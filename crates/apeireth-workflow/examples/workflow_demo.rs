//! R152: workflow demo — 跑一个简单的 sum workflow

use apeireth_workflow::{
    Activity, ActivityInput, ActivityOutput, Workflow, WorkflowContext, WorkflowResult,
    WorkflowRunner,
};
use std::sync::Arc;

struct EchoActivity;
impl Activity for EchoActivity {
    fn execute(&self, input: &ActivityInput) -> Result<ActivityOutput, String> {
        Ok(input.clone())
    }
}

struct AddWorkflow;
impl Workflow for AddWorkflow {
    fn id(&self) -> &str {
        "add"
    }
    fn run(
        &self,
        ctx: &WorkflowContext,
        input: &serde_json::Value,
    ) -> WorkflowResult<serde_json::Value> {
        let a = ctx.execute_activity("echo", serde_json::json!(input["a"]))?;
        let b = ctx.execute_activity("echo", serde_json::json!(input["b"]))?;
        Ok(serde_json::json!({"result": a.as_i64().unwrap() + b.as_i64().unwrap()}))
    }
}

fn main() {
    let mut r = WorkflowRunner::new();
    r.register_activity("echo", Arc::new(EchoActivity));
    r.register_workflow(Arc::new(AddWorkflow));

    let result = r
        .run("add", &serde_json::json!({"a": 10, "b": 20}))
        .unwrap();
    println!("result: {}", result);
    assert_eq!(result["result"], 30);
    println!("history events: {}", r.get_history("add").unwrap().len());
}
