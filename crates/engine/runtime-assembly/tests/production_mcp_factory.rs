//! Controlled stdio acceptance through the default production composition.
#[cfg(target_os = "linux")]
#[path = "support/convergence.rs"]
mod support;
#[cfg(target_os = "linux")]
#[tokio::test]
async fn production_mcp_owns_fresh_processes_and_external_arcs_cannot_revive_stopped_resources() {
    use apeireth_core::{
        kernel::{SessionId, SystemClock, TraceId},
        stored_doc,
    };
    use apeireth_plugin::{resource_factory::module_from_definition, PluginContext};
    use apeireth_runtime::TurnRequest;
    use apeireth_runtime_assembly::{
        ProductionBackends, ProductionModules, ProductionModulesConfig,
    };
    use apeireth_tools_canonical::mcp_bridge::{
        doc_compat, server_list_path, McpPermissionMapping, McpServerConfig, McpServerSpec,
    };
    use serde_json::json;
    use std::{sync::Arc, time::Duration};
    use support::*;
    let directory = tempfile::tempdir().unwrap();
    let pid_file = directory.path().join("server.pid");
    let script = r#"import json,os,sys
open(sys.argv[1], 'w').write(str(os.getpid()))
for line in sys.stdin:
    request=json.loads(line)
    if 'id' not in request: continue
    method=request['method']
    if method=='initialize':
        result={'protocolVersion':request['params']['protocolVersion'],'capabilities':{'tools':{}},'serverInfo':{'name':'controlled-stdio','version':'1'}}
    elif method=='tools/list':
        result={'tools':[{'name':'echo','description':'Controlled echo','inputSchema':{'type':'object','properties':{}},'annotations':{'readOnlyHint':True}}]}
    elif method=='tools/call':
        result={'content':[{'type':'text','text':'production_owned_resource_reply'}],'isError':False}
    else: result={}
    print(json.dumps({'jsonrpc':'2.0','id':request['id'],'result':result}),flush=True)
"#;
    let config = McpServerConfig::new(vec![McpServerSpec::stdio("demo", "python3")
        .with_arg("-u")
        .with_arg("-c")
        .with_arg(script)
        .with_arg(pid_file.to_str().unwrap())]);
    stored_doc::save_single(
        &server_list_path(directory.path()),
        &doc_compat(),
        config,
        stored_doc::DEFAULT_DOC_MODE,
    )
    .unwrap();
    let modules = ProductionModules::build(
        ProductionModulesConfig {
            mcp: true,
            mcp_data_dir: Some(directory.path().into()),
            memory_recall: false,
            memory_writeback: false,
            preference_recall: false,
            self_assessment: false,
            filesystem: false,
            search: false,
            repo: false,
            ..Default::default()
        },
        ProductionBackends::default(),
        Arc::new(SystemClock),
    )
    .unwrap();
    let definition = modules.mcp_definition().unwrap().clone();
    assert!(
        modules.mcp_bridge().is_none(),
        "configuration must not open a process"
    );
    let provider = Provider::new(vec![vec![call("mcp:demo:echo", json!({}))]]);
    let mut runtime = modules
        .register_into(builder(provider.clone()))
        .build()
        .await
        .unwrap();
    let first = definition
        .current_bridge()
        .expect("runtime initialized resources");
    let first_pid = std::fs::read_to_string(&pid_file).unwrap();
    McpPermissionMapping::authorize(
        &mut first.policy().lock().unwrap(),
        &first.catalog().get("mcp:demo:echo").unwrap(),
    );
    runtime
        .execute(TurnRequest::new(SessionId::new(), "production echo"))
        .await
        .unwrap();
    assert!(provider
        .results()
        .iter()
        .any(|result| result.contains("production_owned_resource_reply")));
    let old_tool = first.tools().pop().unwrap();
    runtime.stop_module(&pid("module.mcp")).await.unwrap();
    assert!(first.connections()[0].connect().await.is_err());
    assert!(!old_tool
        .invoke(&call("mcp:demo:echo", json!({})))
        .await
        .is_ok());
    tokio::time::timeout(Duration::from_secs(2), async {
        while std::path::Path::new(&format!("/proc/{}", first_pid.trim())).exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("explicit shutdown must reap while external bridge and tool Arcs remain");
    runtime.unregister_module(&pid("module.mcp")).unwrap();
    let context = PluginContext::new(
        runtime.clock().clone(),
        Arc::new(apeireth_plugin::credentials::NoCredentials),
        TraceId::new(),
    );
    runtime
        .register_module(module_from_definition(definition.clone()), &context)
        .await
        .unwrap();
    let fresh = definition.current_bridge().unwrap();
    assert!(!Arc::ptr_eq(&first, &fresh));
    assert!(
        first.connections()[0].connect().await.is_err(),
        "old external connection cannot reopen new instance"
    );
    McpPermissionMapping::authorize(
        &mut fresh.policy().lock().unwrap(),
        &fresh.catalog().get("mcp:demo:echo").unwrap(),
    );
    provider.enqueue_tools(vec![call("mcp:demo:echo", json!({}))]);
    runtime
        .execute(TurnRequest::new(SessionId::new(), "fresh production echo"))
        .await
        .unwrap();
    for request in provider.requests() {
        apeireth_protocol::canonical::validate_closed_tool_context(&request.messages).unwrap();
    }
    assert!(runtime.shutdown().await.is_empty());
    assert!(fresh.connections()[0]
        .call_tool("echo", json!({}))
        .await
        .is_err());
}
