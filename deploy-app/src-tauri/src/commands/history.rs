use crate::db::{Db, DeployRow};
use tauri::State;

#[tauri::command]
pub fn list_deploys(db: State<'_, Db>, project_path: String, env: String) -> Result<Vec<DeployRow>, String> {
    db.list(&project_path, &env, 100).map_err(|e| format!("{:#}", e))
}

#[tauri::command]
pub fn get_deploy_log(db: State<'_, Db>, id: i64) -> Result<Option<String>, String> {
    db.get_log(id).map_err(|e| format!("{:#}", e))
}
