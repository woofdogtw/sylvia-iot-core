use std::collections::HashMap;

use laboratory::SpecContext;

use sylvia_iot_corelib::server_config::Config as ServerConf;
use sylvia_iot_coremgr::libs::mq::rumqttd;

use super::STATE;
use crate::TestState;

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let _ = state.rumqttd_handles.take();
    let _runtime = state.runtime.as_ref().unwrap();
}

pub fn start_rumqttd(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut state = context.state.borrow_mut();
    let state = state.get_mut(STATE).unwrap();
    let opts = &state.mq_opts.as_ref().unwrap().2;

    let server_conf = ServerConf::default();
    state.rumqttd_handles = Some(rumqttd::start_rumqttd(&server_conf, opts));
    Ok(())
}
