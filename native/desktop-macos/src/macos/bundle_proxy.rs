use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{ClassType, exception, extern_class, msg_send};

extern_class!(
    #[unsafe(super(NSObject))]
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub struct LSBundleProxy;
);

impl LSBundleProxy {
    // https://notprivateapis.com/documentation/notprivateapis/bundleproxyforcurrentprocess
    #[allow(non_snake_case)]
    pub(crate) fn bundleProxyForCurrentProcess() -> Option<Retained<Self>> {
        exception::catch(|| {
            let bundle: Option<Retained<Self>> = unsafe { msg_send![Self::class(), bundleProxyForCurrentProcess] };
            bundle
        })
        .ok()
        .flatten()
    }
}
