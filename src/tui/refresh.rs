use super::app::App;

impl App {
    pub async fn refresh_conversations(&mut self) -> anyhow::Result<()> {
        self.conversations = self.client.get_conversations(100).await?;
        if self.selected_conv_id.is_some()
            && !self.conversations.iter().any(|c| Some(c.id) == self.selected_conv_id)
        {
            self.selected_conv_id = self.conversations.first().map(|c| c.id);
        }
        if self.selected_conv_id.is_none() && !self.conversations.is_empty() {
            self.selected_conv_id = self.conversations.first().map(|c| c.id);
        }
        if let Some(id) = self.selected_conv_id {
            self.messages = self.client.get_messages_page(id, None, 100).await?;
        }
        Ok(())
    }

    pub async fn refresh_metrics(&mut self) -> anyhow::Result<()> {
        self.system_load = Some(crate::backend::get_system_load().await);
        Ok(())
    }

    pub async fn refresh_agents(&mut self) -> anyhow::Result<()> {
        self.agents = self.client.get_all_agents().await?;
        if self.selected_agent_id.is_some()
            && !self.agents.iter().any(|a| Some(a.id) == self.selected_agent_id)
        {
            self.selected_agent_id = self.agents.first().map(|a| a.id);
        }
        if self.selected_agent_id.is_none() && !self.agents.is_empty() {
            self.selected_agent_id = self.agents.first().map(|a| a.id);
        }
        Ok(())
    }

    pub async fn refresh_supervisors(&mut self) -> anyhow::Result<()> {
        self.supervisors = self.client.get_supervisors().await?;
        if self.selected_supervisor_id.is_some()
            && !self.supervisors.iter().any(|s| Some(s.id) == self.selected_supervisor_id)
        {
            self.selected_supervisor_id = self.supervisors.first().map(|s| s.id);
        }
        if self.selected_supervisor_id.is_none() && !self.supervisors.is_empty() {
            self.selected_supervisor_id = self.supervisors.first().map(|s| s.id);
        }
        Ok(())
    }

    pub async fn refresh_tools(&mut self) -> anyhow::Result<()> {
        self.tools = self.client.get_tools().await?;
        if self.selected_tool_id.is_some()
            && !self.tools.iter().any(|t| Some(t.id) == self.selected_tool_id)
        {
            self.selected_tool_id = self.tools.first().map(|t| t.id);
        }
        if self.selected_tool_id.is_none() && !self.tools.is_empty() {
            self.selected_tool_id = self.tools.first().map(|t| t.id);
        }
        Ok(())
    }

    pub async fn refresh_workflows(&mut self) -> anyhow::Result<()> {
        // get_workflows returns a Result — handle both single and list returns
        match self.client.get_workflows().await {
            Ok(list) => self.workflows = list,
            Err(_) => { /* keep existing list */ }
        }
        if self.selected_workflow_id.is_some()
            && !self.workflows.iter().any(|w| Some(w.id) == self.selected_workflow_id)
        {
            self.selected_workflow_id = self.workflows.first().map(|w| w.id);
        }
        if self.selected_workflow_id.is_none() && !self.workflows.is_empty() {
            self.selected_workflow_id = self.workflows.first().map(|w| w.id);
        }
        Ok(())
    }

    pub async fn refresh_computer(&mut self) -> anyhow::Result<()> {
        self.computer_state = self.client.get_computer_supervision_state().await.ok();
        Ok(())
    }

    pub async fn refresh_settings_map(&mut self) -> anyhow::Result<()> {
        self.settings_map = self.client.get_all_settings().await;
        Ok(())
    }
}
