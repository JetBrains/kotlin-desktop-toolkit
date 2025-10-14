use objc2::runtime::NSObject;
use objc2::{exception, extern_class, msg_send, ClassType};
use objc2::rc::Retained;

extern_class!(
    #[unsafe(super(NSObject))]
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub struct LSBundleProxy;
);

impl LSBundleProxy {
    // https://notprivateapis.com/documentation/notprivateapis/bundleproxyforcurrentprocess
    pub(crate) fn bundleProxyForCurrentProcess() -> Option<Retained<Self>> {
        exception::catch(|| {
            let bundle: Option<Retained<LSBundleProxy>> = unsafe { msg_send![LSBundleProxy::class(), bundleProxyForCurrentProcess] };
            bundle
        }).ok().flatten()
    }
}