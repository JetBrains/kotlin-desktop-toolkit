/*
SPDX-License-Identifier: Apache-2.0 OR MIT

Copyright 2022 The Arboard contributors

The project to which this file belongs is licensed under either of
the Apache 2.0 or the MIT license at the licensee's choice. The terms
and conditions of the chosen license apply to this file.
*/

// More info about using the clipboard on X11:
// https://tronche.com/gui/x/icccm/sec-2.html#s-2.6
// https://freedesktop.org/wiki/ClipboardManager/

use std::{
    cell::RefCell,
    collections::{HashMap, hash_map::Entry},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    thread_local,
    time::{Duration, Instant},
};

use super::{
    Error, LinuxClipboardKind, WaitConfig,
    common::{ScopeGuard, into_unknown},
};
use log::{error, info, trace, warn};
use parking_lot::{Condvar, Mutex, MutexGuard, RwLock};
use x11rb::{
    COPY_DEPTH_FROM_PARENT, COPY_FROM_PARENT, NONE,
    connection::Connection,
    protocol::{
        Event,
        xproto::{
            Atom, AtomEnum, ConnectionExt as _, CreateWindowAux, EventMask, PropMode, Property, PropertyNotifyEvent,
            SELECTION_NOTIFY_EVENT, SelectionNotifyEvent, SelectionRequestEvent, Time, WindowClass,
        },
    },
    rust_connection::RustConnection,
    wrapper::ConnectionExt as _,
};

type Result<T, E = Error> = std::result::Result<T, E>;

static CLIPBOARD: Mutex<Option<GlobalClipboard>> = parking_lot::const_mutex(None);

x11rb::atom_manager! {
    pub Atoms: AtomCookies {
        CLIPBOARD,
        PRIMARY,

        CLIPBOARD_MANAGER,
        SAVE_TARGETS,
        TARGETS,
        ATOM,
        INCR,

        UTF8_STRING,
        UTF8_MIME_0: b"text/plain;charset=utf-8",
        UTF8_MIME_1: b"text/plain;charset=UTF-8",
        // Text in ISO Latin-1 encoding
        // See: https://tronche.com/gui/x/icccm/sec-2.html#s-2.6.2
        STRING,
        // Text in unknown encoding
        // See: https://tronche.com/gui/x/icccm/sec-2.html#s-2.6.2
        TEXT,
        TEXT_MIME_UNKNOWN: b"text/plain",

        HTML: b"text/html",
        URI_LIST: b"text/uri-list",

        PNG_MIME: b"image/png",

        // This is just some random name for the property on our window, into which
        // the clipboard owner writes the data we requested.
        ARBOARD_CLIPBOARD,
    }
}

thread_local! {
    static ATOM_NAME_CACHE: RefCell<HashMap<Atom, &'static str>> = Default::default();
}

// Some clipboard items, like images, may take a very long time to produce a
// `SelectionNotify`. Multiple seconds long.
const LONG_TIMEOUT_DUR: Duration = Duration::from_millis(4000);
const SHORT_TIMEOUT_DUR: Duration = Duration::from_millis(10);

#[derive(Debug, PartialEq, Eq)]
enum ManagerHandoverState {
    Idle,
    InProgress,
    Finished,
}

struct GlobalClipboard {
    inner: Arc<Inner>,

    /// Join handle to the thread which serves selection requests.
    server_handle: JoinHandle<()>,
}

struct XContext {
    conn: RustConnection,
    win_id: u32,
}

struct Inner {
    /// The context for the thread which serves clipboard read
    /// requests coming to us.
    server: XContext,
    atoms: Atoms,
    get_data_transfer_data: Mutex<Box<dyn Fn(LinuxClipboardKind, &str) -> Option<Vec<u8>> + Send>>,

    clipboard: Selection,
    primary: Selection,

    handover_state: Mutex<ManagerHandoverState>,
    handover_cv: Condvar,

    serve_stopped: AtomicBool,
}

impl XContext {
    fn new() -> Result<Self> {
        // create a new connection to an X11 server
        let (conn, screen_num): (RustConnection, _) =
            RustConnection::connect(None).map_err(|_| Error::unknown("X11 server connection timed out because it was unreachable"))?;
        let screen = conn.setup().roots.get(screen_num).ok_or(Error::unknown("no screen found"))?;
        let win_id = conn.generate_id().map_err(into_unknown)?;

        let event_mask =
            // Just in case that some program reports SelectionNotify events
            // with XCB_EVENT_MASK_PROPERTY_CHANGE mask.
            EventMask::PROPERTY_CHANGE |
                // To receive DestroyNotify event and stop the message loop.
                EventMask::STRUCTURE_NOTIFY;
        // create the window
        conn.create_window(
            // copy as much as possible from the parent, because no other specific input is needed
            COPY_DEPTH_FROM_PARENT,
            win_id,
            screen.root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::COPY_FROM_PARENT,
            COPY_FROM_PARENT,
            // don't subscribe to any special events because we are requesting everything we need ourselves
            &CreateWindowAux::new().event_mask(event_mask),
        )
        .map_err(into_unknown)?;
        conn.flush().map_err(into_unknown)?;

        Ok(Self { conn, win_id })
    }
}

#[derive(Default)]
struct MimeData {
    name: String,
    format: Atom,
}

#[derive(Default)]
struct Selection {
    data: RwLock<Option<Vec<MimeData>>>,
    /// Mutex around when this selection was last changed by us
    /// for both use with the below condvar and logging.
    mutex: Mutex<Option<Instant>>,
    /// A condvar that is notified when the contents of this clipboard are changed.
    ///
    /// This is associated with `Self::mutex`.
    data_changed: Condvar,
}

enum ReadSelNotifyResult {
    GotData(Vec<u8>),
    IncrStarted,
    EventNotRecognized,
}

impl Inner {
    fn new(get_data_transfer_data: Box<dyn Fn(LinuxClipboardKind, &str) -> Option<Vec<u8>> + Send>) -> Result<Self> {
        let server = XContext::new()?;
        let atoms = Atoms::new(&server.conn).map_err(into_unknown)?.reply().map_err(into_unknown)?;

        Ok(Self {
            server,
            atoms,
            get_data_transfer_data: Mutex::new(get_data_transfer_data),
            clipboard: Selection::default(),
            primary: Selection::default(),
            handover_state: Mutex::new(ManagerHandoverState::Idle),
            handover_cv: Condvar::new(),
            serve_stopped: AtomicBool::new(false),
        })
    }

    /// Performs a "clear" operation on the clipboard, which is implemented by
    /// relinquishing the selection to revert its owner to `None`. This gracefully
    /// and comformly informs the X server and any clipboard managers that the
    /// data was no longer valid and won't be offered from our window anymore.
    ///
    /// See `ask_clipboard_manager_to_request_our_data` for more details on why
    /// this is important and specification references.
    fn clear(&self, selection: LinuxClipboardKind) -> Result<()> {
        let selection = self.atom_of(selection);

        self.server
            .conn
            .set_selection_owner(NONE, selection, Time::CURRENT_TIME)
            .map_err(into_unknown)?;

        self.server.conn.flush().map_err(into_unknown)
    }

    fn write(&self, data: Vec<MimeData>, clipboard_selection: LinuxClipboardKind, wait: WaitConfig) -> Result<()> {
        if self.serve_stopped.load(Ordering::Relaxed) {
            return Err(Error::unknown(
                "The clipboard handler thread seems to have stopped. Logging messages may reveal the cause. (See the `log` crate.)",
            ));
        }

        let server_win = self.server.win_id;

        // Just setting the data, and the `serve_requests` will take care of the rest.
        let selection = self.selection_of(clipboard_selection);
        let mut data_guard = selection.data.write();
        *data_guard = Some(data);

        // ICCCM version 2, section 2.6.1.3 states that we should re-assert ownership whenever data
        // changes.
        self.server
            .conn
            .set_selection_owner(server_win, self.atom_of(clipboard_selection), Time::CURRENT_TIME)
            .map_err(|_| Error::ClipboardOccupied)?;

        self.server.conn.flush().map_err(into_unknown)?;

        // Lock the mutex to both ensure that no wakers of `data_changed` can wake us between
        // dropping the `data_guard` and calling `wait[_for]` and that we don't we wake other
        // threads in that position.
        let mut guard = selection.mutex.lock();
        // Record the time we modify the selection.
        *guard = Some(Instant::now());

        // Notify any existing waiting threads that we have changed the data in the selection.
        // It is important that the mutex is locked to prevent this notification getting lost.
        selection.data_changed.notify_all();

        match wait {
            WaitConfig::None => {}
            WaitConfig::Forever => {
                drop(data_guard);
                selection.data_changed.wait(&mut guard);
            }

            WaitConfig::Until(deadline) => {
                drop(data_guard);
                selection.data_changed.wait_until(&mut guard, deadline);
            }
        }

        Ok(())
    }

    fn read(&self, format: Atom, selection: LinuxClipboardKind) -> Result<Vec<u8>> {
        // if let Some(data) = self.data.read().clone() {
        //     return Ok(data)
        // }
        let reader = XContext::new()?;

        trace!("Trying to get the clipboard data.");
        self.read_single(&reader, selection, format)
    }

    fn read_single(&self, reader: &XContext, selection: LinuxClipboardKind, target_format: Atom) -> Result<Vec<u8>> {
        // Delete the property so that we can detect (using property notify)
        // when the selection owner receives our request.
        reader
            .conn
            .delete_property(reader.win_id, self.atoms.ARBOARD_CLIPBOARD)
            .map_err(into_unknown)?;

        // request to convert the clipboard selection to our data type(s)
        reader
            .conn
            .convert_selection(
                reader.win_id,
                self.atom_of(selection),
                target_format,
                self.atoms.ARBOARD_CLIPBOARD,
                Time::CURRENT_TIME,
            )
            .map_err(into_unknown)?;
        reader.conn.sync().map_err(into_unknown)?;

        trace!("Finished `convert_selection`");

        let mut incr_data: Vec<u8> = Vec::new();
        let mut using_incr = false;

        let mut timeout_end = Instant::now() + LONG_TIMEOUT_DUR;

        while Instant::now() < timeout_end {
            let event = reader.conn.poll_for_event().map_err(into_unknown)?;
            let event = match event {
                Some(e) => e,
                None => {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                }
            };
            match event {
                // The first response after requesting a selection.
                Event::SelectionNotify(event) => {
                    trace!("Read SelectionNotify");
                    let result = self.handle_read_selection_notify(reader, target_format, &mut using_incr, &mut incr_data, event)?;
                    match result {
                        ReadSelNotifyResult::GotData(data) => return Ok(data),
                        ReadSelNotifyResult::IncrStarted => {
                            // This means we received an indication that an the
                            // data is going to be sent INCRementally. Let's
                            // reset our timeout.
                            timeout_end += SHORT_TIMEOUT_DUR;
                        }
                        ReadSelNotifyResult::EventNotRecognized => (),
                    }
                }
                // If the previous SelectionNotify event specified that the data
                // will be sent in INCR segments, each segment is transferred in
                // a PropertyNotify event.
                Event::PropertyNotify(event) => {
                    let result =
                        self.handle_read_property_notify(reader, target_format, using_incr, &mut incr_data, &mut timeout_end, event)?;
                    if result {
                        return Ok(incr_data);
                    }
                }
                _ => trace!("An unexpected event arrived while reading the clipboard."),
            }
        }
        info!("Time-out hit while reading the clipboard.");
        Err(Error::ContentNotAvailable)
    }

    fn atom_of(&self, selection: LinuxClipboardKind) -> Atom {
        match selection {
            LinuxClipboardKind::Clipboard => self.atoms.CLIPBOARD,
            LinuxClipboardKind::Primary => self.atoms.PRIMARY,
        }
    }

    fn selection_of(&self, selection: LinuxClipboardKind) -> &Selection {
        match selection {
            LinuxClipboardKind::Clipboard => &self.clipboard,
            LinuxClipboardKind::Primary => &self.primary,
        }
    }

    fn kind_of(&self, atom: Atom) -> Option<LinuxClipboardKind> {
        match atom {
            a if a == self.atoms.CLIPBOARD => Some(LinuxClipboardKind::Clipboard),
            a if a == self.atoms.PRIMARY => Some(LinuxClipboardKind::Primary),
            _ => None,
        }
    }

    fn is_owner(&self, selection: LinuxClipboardKind) -> Result<bool> {
        let current = self
            .server
            .conn
            .get_selection_owner(self.atom_of(selection))
            .map_err(into_unknown)?
            .reply()
            .map_err(into_unknown)?
            .owner;

        Ok(current == self.server.win_id)
    }

    fn atom_name(&self, atom: Atom) -> Result<String> {
        String::from_utf8(
            self.server
                .conn
                .get_atom_name(atom)
                .map_err(into_unknown)?
                .reply()
                .map_err(into_unknown)?
                .name,
        )
        .map_err(into_unknown)
    }
    fn atom_name_dbg(&self, atom: Atom) -> &'static str {
        ATOM_NAME_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            match cache.entry(atom) {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => {
                    let s = self
                        .atom_name(atom)
                        .map(|s| Box::leak(s.into_boxed_str()) as &str)
                        .unwrap_or("FAILED-TO-GET-THE-ATOM-NAME");
                    entry.insert(s);
                    s
                }
            }
        })
    }

    fn handle_read_selection_notify(
        &self,
        reader: &XContext,
        target_format: u32,
        using_incr: &mut bool,
        incr_data: &mut Vec<u8>,
        event: SelectionNotifyEvent,
    ) -> Result<ReadSelNotifyResult> {
        // The property being set to NONE means that the `convert_selection`
        // failed.

        // According to: https://tronche.com/gui/x/icccm/sec-2.html#s-2.4
        // the target must be set to the same as what we requested.
        if event.property == NONE || event.target != target_format {
            return Err(Error::ContentNotAvailable);
        }
        if self.kind_of(event.selection).is_none() {
            info!("Received a SelectionNotify for a selection other than CLIPBOARD, PRIMARY or SECONDARY. This is unexpected.");
            return Ok(ReadSelNotifyResult::EventNotRecognized);
        }
        if *using_incr {
            warn!("Received a SelectionNotify while already expecting INCR segments.");
            return Ok(ReadSelNotifyResult::EventNotRecognized);
        }
        // request the selection
        let mut reply = reader
            .conn
            .get_property(true, event.requestor, event.property, event.target, 0, u32::MAX / 4)
            .map_err(into_unknown)?
            .reply()
            .map_err(into_unknown)?;

        // trace!("Property.type: {:?}", self.atom_name(reply.type_));

        // we found something
        if reply.type_ == target_format {
            Ok(ReadSelNotifyResult::GotData(reply.value))
        } else if reply.type_ == self.atoms.INCR {
            // Note that we call the get_property again because we are
            // indicating that we are ready to receive the data by deleting the
            // property, however deleting only works if the type matches the
            // property type. But the type didn't match in the previous call.
            reply = reader
                .conn
                .get_property(true, event.requestor, event.property, self.atoms.INCR, 0, u32::MAX / 4)
                .map_err(into_unknown)?
                .reply()
                .map_err(into_unknown)?;
            trace!("Receiving INCR segments");
            *using_incr = true;
            if reply.value_len == 4 {
                let min_data_len = reply.value32().and_then(|mut vals| vals.next()).unwrap_or(0);
                incr_data.reserve(min_data_len as usize);
            }
            Ok(ReadSelNotifyResult::IncrStarted)
        } else {
            // this should never happen, we have sent a request only for supported types
            Err(Error::unknown("incorrect type received from clipboard"))
        }
    }

    /// Returns Ok(true) when the incr_data is ready
    fn handle_read_property_notify(
        &self,
        reader: &XContext,
        target_format: u32,
        using_incr: bool,
        incr_data: &mut Vec<u8>,
        timeout_end: &mut Instant,
        event: PropertyNotifyEvent,
    ) -> Result<bool> {
        if event.atom != self.atoms.ARBOARD_CLIPBOARD || event.state != Property::NEW_VALUE {
            return Ok(false);
        }
        if !using_incr {
            // This must mean the selection owner received our request, and is
            // now preparing the data
            return Ok(false);
        }
        let reply = reader
            .conn
            .get_property(true, event.window, event.atom, target_format, 0, u32::MAX / 4)
            .map_err(into_unknown)?
            .reply()
            .map_err(into_unknown)?;

        // trace!("Received segment. value_len {}", reply.value_len,);
        if reply.value_len == 0 {
            // This indicates that all the data has been sent.
            return Ok(true);
        }
        incr_data.extend(reply.value);

        // Let's reset our timeout, since we received a valid chunk.
        *timeout_end = Instant::now() + SHORT_TIMEOUT_DUR;

        // Not yet complete
        Ok(false)
    }

    fn handle_selection_request(&self, event: SelectionRequestEvent) -> Result<()> {
        let selection = match self.kind_of(event.selection) {
            Some(kind) => kind,
            None => {
                warn!("Received a selection request to a selection other than the CLIPBOARD, PRIMARY or SECONDARY. This is unexpected.");
                return Ok(());
            }
        };

        let success;
        // we are asked for a list of supported conversion targets
        if event.target == self.atoms.TARGETS {
            trace!("Handling TARGETS, dst property is {}", self.atom_name_dbg(event.property));

            let data = self.selection_of(selection).data.read();
            let data_targets = if let Some(data_list) = &*data {
                // Estimation based on current data types, plus the other UTF-8 ones, plus `SAVE_TARGETS`.
                let mut targets = Vec::with_capacity(data_list.len() + 3);

                for data in data_list {
                    targets.push(data.format);
                    if data.format == self.atoms.UTF8_STRING {
                        // When we are storing a UTF8 string,
                        // add all equivalent formats to the supported targets
                        targets.push(self.atoms.UTF8_MIME_0);
                        targets.push(self.atoms.UTF8_MIME_1);
                    }
                }
                targets
            } else {
                // If there's no data, we advertise an empty list of targets.
                Vec::with_capacity(2)
            };

            let mut targets = data_targets;
            targets.push(self.atoms.TARGETS);

            // NB: `SAVE_TARGETS` in this context is a marker atom which informs the clipboard manager
            // we support this operation and _may_ use it in the future. To try and keep the manager's
            // expectations/assumptions (if any) about when we will invoke this handoff, we go ahead and
            // skip advertising support for the save operation entirely when the data was marked as
            // sensitive.
            //
            // Note that even if we don't advertise it, some managers may respond to it anyways so this is
            // only half of exclusion handling. See `ask_clipboard_manager_to_request_our_data` for more.
            targets.push(self.atoms.SAVE_TARGETS);

            self.server
                .conn
                .change_property32(
                    PropMode::REPLACE,
                    event.requestor,
                    event.property,
                    // TODO: change to `AtomEnum::ATOM`
                    self.atoms.ATOM,
                    &targets,
                )
                .map_err(into_unknown)?;
            self.server.conn.flush().map_err(into_unknown)?;
            success = true;
        } else {
            trace!("Handling request for (probably) the clipboard contents.");
            let data = self.selection_of(selection).data.read();
            if let Some(data_list) = &*data {
                success = match data_list.iter().find(|d| d.format == event.target) {
                    Some(data) => {
                        if let Some(content) = self.get_data_transfer_data.lock()(selection, &data.name) {
                            self.server
                                .conn
                                .change_property8(PropMode::REPLACE, event.requestor, event.property, event.target, &content)
                                .map_err(into_unknown)?;
                            self.server.conn.flush().map_err(into_unknown)?;
                            true
                        } else {
                            false
                        }
                    }
                    None => false,
                };
            } else {
                // This must mean that we lost ownership of the data
                // since the other side requested the selection.
                // Let's respond with the property set to none.
                success = false;
            }
        }
        // on failure we notify the requester of it
        let property = if success { event.property } else { AtomEnum::NONE.into() };
        // tell the requestor that we finished sending data
        self.server
            .conn
            .send_event(
                false,
                event.requestor,
                EventMask::NO_EVENT,
                SelectionNotifyEvent {
                    response_type: SELECTION_NOTIFY_EVENT,
                    sequence: event.sequence,
                    time: event.time,
                    requestor: event.requestor,
                    selection: event.selection,
                    target: event.target,
                    property,
                },
            )
            .map_err(into_unknown)?;

        self.server.conn.flush().map_err(into_unknown)
    }

    // pub(crate) fn get_supported_formats(&self, selection: LinuxClipboardKind) -> Result<Vec<String>> {
    //     if self.is_owner(selection)? {
    //         let data = self.selection_of(selection).data.read();
    //         if let Some(data_list) = &*data {
    //             Ok(data_list.iter().map(|e| e.name.clone()).collect())
    //         } else {
    //             Ok(Vec::new())
    //         }
    //     } else {
    //         // TODO
    //         Ok(Vec::new())
    //     }
    // }

    fn ask_clipboard_manager_to_request_our_data(&self) -> Result<()> {
        if self.server.win_id == 0 {
            // This shouldn't really ever happen but let's just check.
            error!("The server's window id was 0. This is unexpected");
            return Ok(());
        }

        // Per the `ClipboardManager` specification, only the `CLIPBOARD` target is
        // to be saved from other X clients, so if the caller set the `Primary` (or `Secondary`) clipboard,
        // we wouldn't expect any clipboard manager to save that anyway.
        let selection = LinuxClipboardKind::Clipboard;

        if !self.is_owner(selection)? {
            // We are not owning the clipboard, nothing to do.
            return Ok(());
        }

        if self.selection_of(selection).data.read().is_none() {
            // If we don't have any data, there's nothing to do.
            return Ok(());
        }

        // It's important that we lock the state before sending the request
        // because we don't want the request server thread to lock the state
        // after the request but before we can lock it here.
        let mut handover_state = self.handover_state.lock();

        trace!("Sending the data to the clipboard manager");
        self.server
            .conn
            .convert_selection(
                self.server.win_id,
                self.atoms.CLIPBOARD_MANAGER,
                self.atoms.SAVE_TARGETS,
                self.atoms.ARBOARD_CLIPBOARD,
                Time::CURRENT_TIME,
            )
            .map_err(into_unknown)?;
        self.server.conn.flush().map_err(into_unknown)?;

        *handover_state = ManagerHandoverState::InProgress;
        let max_handover_duration = Duration::from_millis(100);

        // Note that we are using a parking_lot condvar here, which doesn't wake up
        // spuriously
        let result = self.handover_cv.wait_for(&mut handover_state, max_handover_duration);

        if *handover_state == ManagerHandoverState::Finished {
            return Ok(());
        }
        if result.timed_out() {
            warn!("Could not hand the clipboard contents over to the clipboard manager. The request timed out.");
            return Ok(());
        }

        unreachable!("This is a bug! The handover was not finished and the condvar didn't time out, yet the condvar wait ended.")
    }
}

fn serve_requests(context: Arc<Inner>) -> Result<(), Box<dyn std::error::Error>> {
    fn handover_finished(clip: &Arc<Inner>, mut handover_state: MutexGuard<ManagerHandoverState>) {
        trace!("Finishing clipboard manager handover.");
        *handover_state = ManagerHandoverState::Finished;

        // Not sure if unlocking the mutex is necessary here but better safe than sorry.
        drop(handover_state);

        clip.handover_cv.notify_all();
    }

    trace!("Started serve requests thread.");

    let _guard = ScopeGuard::new(|| {
        context.serve_stopped.store(true, Ordering::Relaxed);
    });

    let mut written = false;
    let mut notified = false;

    loop {
        match context.server.conn.wait_for_event().map_err(into_unknown)? {
            Event::DestroyNotify(_) => {
                // This window is being destroyed.
                trace!("Clipboard server window is being destroyed x_x");
                return Ok(());
            }
            Event::SelectionClear(event) => {
                // TODO: check if this works
                // Someone else has new content in the clipboard, so it is
                // notifying us that we should delete our data now.
                trace!("Somebody else owns the clipboard now");

                if let Some(selection) = context.kind_of(event.selection) {
                    let selection = context.selection_of(selection);
                    let mut data_guard = selection.data.write();
                    *data_guard = None;

                    // It is important that this mutex is locked at the time of calling
                    // `notify_all` to prevent notifications getting lost in case the sleeping
                    // thread has unlocked its `data_guard` and is just about to sleep.
                    // It is also important that the RwLock is kept write-locked for the same
                    // reason.
                    let _guard = selection.mutex.lock();
                    selection.data_changed.notify_all();
                }
            }
            Event::SelectionRequest(event) => {
                trace!(
                    "SelectionRequest - selection is: {}, target is {}",
                    context.atom_name_dbg(event.selection),
                    context.atom_name_dbg(event.target),
                );
                // Someone is requesting the clipboard content from us.
                if let Err(e) = context.handle_selection_request(event) {
                    error!("Failed to handle selection request: {e}");
                    continue;
                }

                // if we are in the progress of saving to the clipboard manager
                // make sure we save that we have finished writing
                let handover_state = context.handover_state.lock();
                if *handover_state == ManagerHandoverState::InProgress {
                    // Only set written, when the actual contents were written,
                    // not just a response to what TARGETS we have.
                    if event.target != context.atoms.TARGETS {
                        trace!("The contents were written to the clipboard manager.");
                        written = true;
                        // if we have written and notified, make sure to notify that we are done
                        if notified {
                            handover_finished(&context, handover_state);
                        }
                    }
                }
            }
            Event::SelectionNotify(event) => {
                // We've requested the clipboard content and this is the answer.
                // Considering that this thread is not responsible for reading
                // clipboard contents, this must come from the clipboard manager
                // signaling that the data was handed over successfully.
                if event.selection != context.atoms.CLIPBOARD_MANAGER {
                    error!(
                        "Received a `SelectionNotify` from a selection other than the CLIPBOARD_MANAGER. This is unexpected in this thread."
                    );
                    continue;
                }
                let handover_state = context.handover_state.lock();
                if *handover_state == ManagerHandoverState::InProgress {
                    // Note that some clipboard managers send a selection notify
                    // before even sending a request for the actual contents.
                    // (That's why we use the "notified" & "written" flags)
                    trace!("The clipboard manager indicated that it's done requesting the contents from us.");
                    notified = true;

                    // One would think that we could also finish if the property
                    // here is set 0, because that indicates failure. However
                    // this is not the case; for example on KDE plasma 5.18, we
                    // immediately get a SelectionNotify with property set to 0,
                    // but following that, we also get a valid SelectionRequest
                    // from the clipboard manager.
                    if written {
                        handover_finished(&context, handover_state);
                    }
                }
            }
            _event => {
                // May be useful for debugging but nothing else really.
                // trace!("Received unwanted event: {:?}", event);
            }
        }
    }
}

pub(crate) struct Clipboard {
    inner: Arc<Inner>,
}

impl Clipboard {
    pub(crate) fn new(get_data_transfer_data: Box<dyn Fn(LinuxClipboardKind, &str) -> Option<Vec<u8>> + Send>) -> Result<Self> {
        let mut global_cb = CLIPBOARD.lock();
        if let Some(global_cb) = &*global_cb {
            return Ok(Self {
                inner: Arc::clone(&global_cb.inner),
            });
        }
        // At this point we know that the clipboard does not exist.
        let ctx = Arc::new(Inner::new(get_data_transfer_data)?);
        let join_handle;
        {
            let ctx = Arc::clone(&ctx);
            join_handle = std::thread::spawn(move || {
                if let Err(error) = serve_requests(ctx) {
                    error!("Worker thread errored with: {}", error);
                }
            });
        }
        *global_cb = Some(GlobalClipboard {
            inner: Arc::clone(&ctx),
            server_handle: join_handle,
        });
        Ok(Self { inner: ctx })
    }

    pub(crate) fn clear(&self, selection: LinuxClipboardKind) -> Result<()> {
        self.inner.clear(selection)
    }

    pub(crate) fn get_custom_format(&self, selection: LinuxClipboardKind, mime_type: &str) -> Result<Vec<u8>> {
        let atom = self
            .inner
            .server
            .conn
            .intern_atom(true, mime_type.as_bytes())
            .map_err(|e| Error::Unknown {
                description: e.to_string(),
            })?
            .reply()
            .map_err(|e| Error::Unknown {
                description: e.to_string(),
            })?
            .atom;
        let bytes = self.inner.read(atom, selection)?;
        Ok(bytes)
    }

    // pub(crate) fn get_supported_formats(&self, selection: LinuxClipboardKind) -> Result<Vec<String>> {
    //     self.inner.get_supported_formats(selection)
    // }

    pub(crate) fn commit_all_formats(&self, formats: Vec<String>, selection: LinuxClipboardKind, wait: WaitConfig) -> Result<()> {
        let mut data = Vec::new();

        for mime_type in formats {
            // Register a custom MIME type as X11 atom
            let atom = self
                .inner
                .server
                .conn
                .intern_atom(false, mime_type.as_bytes())
                .map_err(|e| Error::Unknown {
                    description: e.to_string(),
                })?
                .reply()
                .map_err(|e| Error::Unknown {
                    description: e.to_string(),
                })?
                .atom;
            data.push(MimeData {
                name: mime_type,
                format: atom,
            });
        }

        self.inner.write(data, selection, wait)
    }
}

impl Drop for Clipboard {
    fn drop(&mut self) {
        // There are always at least 3 owners:
        // the global, the server thread, and one `Clipboard::inner`
        const MIN_OWNERS: usize = 3;

        // We start with locking the global guard to prevent race
        // conditions below.
        let mut global_cb = CLIPBOARD.lock();
        if Arc::strong_count(&self.inner) == MIN_OWNERS {
            // If the are the only owners of the clipboard are ourselves and
            // the global object, then we should destroy the global object,
            // and send the data to the clipboard manager

            if let Err(e) = self.inner.ask_clipboard_manager_to_request_our_data() {
                error!("Could not hand the clipboard data over to the clipboard manager: {}", e);
            }
            let global_cb = global_cb.take();
            if let Err(e) = self.inner.server.conn.destroy_window(self.inner.server.win_id) {
                error!("Failed to destroy the clipboard window. Error: {}", e);
                return;
            }
            if let Err(e) = self.inner.server.conn.flush() {
                error!("Failed to flush the clipboard window. Error: {}", e);
                return;
            }
            if let Some(global_cb) = global_cb {
                let GlobalClipboard { inner, server_handle } = global_cb;
                drop(inner);

                if let Err(e) = server_handle.join() {
                    // Let's try extracting the error message
                    let message;
                    if let Some(msg) = e.downcast_ref::<&'static str>() {
                        message = Some((*msg).to_string());
                    } else if let Some(msg) = e.downcast_ref::<String>() {
                        message = Some(msg.clone());
                    } else {
                        message = None;
                    }
                    if let Some(message) = message {
                        error!("The clipboard server thread panicked. Panic message: '{}'", message,);
                    } else {
                        error!("The clipboard server thread panicked.");
                    }
                }

                // By this point we've dropped the Global's reference to `Inner` and the background
                // thread has exited which means it also dropped its reference. Therefore `self.inner` should
                // be the last strong count.
                //
                // Note: The following is all best effort and is only for logging. Nothing is guaranteed to execute
                // or log.
                #[cfg(debug_assertions)]
                if let Some(inner) = Arc::get_mut(&mut self.inner) {
                    use std::io::IsTerminal;

                    let mut change_timestamps = Vec::with_capacity(2);
                    let mut collect_changed = |sel: &mut Mutex<Option<Instant>>| {
                        if let Some(changed) = sel.get_mut() {
                            change_timestamps.push(*changed);
                        }
                    };

                    collect_changed(&mut inner.clipboard.mutex);
                    collect_changed(&mut inner.primary.mutex);

                    change_timestamps.sort();
                    if let Some(last) = change_timestamps.last() {
                        let elapsed = last.elapsed().as_millis();
                        // This number has no meaning, its just a guess for how long
                        // might be reasonable to give a clipboard manager a chance to
                        // save contents based ~roughly on the handoff timeout.
                        if elapsed > 100 {
                            return;
                        }

                        // If the app isn't running in a terminal don't print, use log instead.
                        // Printing has a higher chance of being seen though, so its our default.
                        // Its also close enough to a `debug_assert!` that it shouldn't come across strange.
                        let msg = format!(
                            "Clipboard was dropped very quickly after writing ({elapsed}ms); clipboard managers may not have seen the contents\nConsider keeping `Clipboard` in more persistent state somewhere or keeping the contents alive longer using `SetLinuxExt` and/or threads."
                        );
                        if std::io::stderr().is_terminal() {
                            eprintln!("{msg}");
                        } else {
                            warn!("{msg}");
                        }
                    }
                }
            }
        }
    }
}
