use std::process::Command;

pub async fn capture_accessibility_tree() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        crate::engine::tools::os_tool::accessibility::windows_accessibility_state_json(
            "vision_accessibility",
            &serde_json::Value::Null
        )
        .await
        .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "linux")]
    {
        let py_script = r#"
import sys, json
try:
    import atspi
    def dump_node(acc):
        if not acc: return None
        node = {"name": acc.name, "role": acc.getRoleName()}
        try:
            ext = acc.queryComponent()
            if ext:
                bbox = ext.getExtents(atspi.XY_SCREEN)
                node["bbox"] = [bbox.x, bbox.y, bbox.width, bbox.height]
        except: pass
        children = []
        for i in range(acc.childCount):
            child = acc.getChildAtIndex(i)
            c_node = dump_node(child)
            if c_node: children.append(c_node)
        if children: node["children"] = children
        return node

    desktop = atspi.getDesktop(0)
    apps = []
    for app in desktop:
        if app:
            data = dump_node(app)
            if data: apps.append(data)
    print(json.dumps(apps))
except Exception as e:
    print(json.dumps({"error": str(e)}))
"#;
        let mut child = Command::new("python3")
            .args(["-c", py_script])
            .output();
        match child {
            Ok(output) if output.status.success() => {
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            _ => Err("Failed to execute Linux accessibility scraper".to_string()),
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Ok("[]".to_string())
    }
}
