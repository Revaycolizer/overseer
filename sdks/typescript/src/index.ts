/**
 * Overseer Client — TypeScript SDK for the Overseer AI orchestration platform.
 *
 * @example
 * ```typescript
 * import { OverseerClient } from 'overseer-client';
 *
 * const client = new OverseerClient({ baseUrl: 'http://localhost:8080' });
 *
 * // List conversations
 * const conversations = await client.conversations.list();
 * console.log(conversations);
 *
 * // List agents
 * const agents = await client.agents.list();
 * console.log(agents);
 * ```
 */

export { OverseerClient } from './client';
export type { OverseerClientOptions } from './client';
export {
  ConversationsAPI,
  MessagesAPI,
  AgentsAPI,
  ToolsAPI,
  SupervisorsAPI,
  WorkflowsAPI,
  WorkItemsAPI,
  ArtifactsAPI,
  PlannersAPI,
  SupervisionAPI,
  VersionsAPI,
  BrowserAPI,
  CredentialsAPI,
  TemplatesAPI,
  SettingsAPI,
  SystemAPI,
} from './client';

export {
  ApiError,
} from './types';

export type {
  // Auth
  UserProfile,
  AuthResponse,
  // Conversations
  Conversation,
  ConversationSearchResult,
  // Messages
  Message,
  Attachment,
  // Agents
  AgentInfo,
  ToolInfo,
  // Supervisors
  Supervisor,
  // Workflows
  Workflow,
  WorkflowStep,
  WorkflowRun,
  WorkflowStepRun,
  // Work Items
  WorkItem,
  WorkItemEvent,
  // Artifacts
  Artifact,
  // Planners
  Planner,
  // Version Control
  EntityVersion,
  // Health / System
  RuntimeHealth,
  SystemLoad,
  RuntimeRegistryStatus,
  // Supervision
  ComputerSupervisionState,
  // Credentials
  CredentialSummary,
  // Workflow Templates
  WorkflowTemplate,
  // Browser
  BrowserSessionInfo,
} from './types';
