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
        // Ensure selection is valid
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
}
