/** TypeScript types matching all Overseer REST API types. */

// ── Auth ──────────────────────────────────────────────────────────────

export interface UserProfile {
  id: number;
  username: string;
  display_name: string;
}

export interface AuthResponse {
  token: string;
  user: UserProfile;
}

// ── Conversations ─────────────────────────────────────────────────────

export interface Conversation {
  id: number;
  title: string;
  created_at: string;
  updated_at: string;
}

export interface ConversationSearchResult {
  conversation_id: number;
  message_id: number;
  snippet: string;
}

// ── Messages & Attachments ────────────────────────────────────────────

export interface Message {
  id: number;
  conversation_id: number;
  role: string;
  content: string;
  created_at: string;
  hide_evidence: boolean;
}

export interface Attachment {
  id: number;
  message_id: number;
  filename: string;
  file_path: string;
  content_type: string;
  size: number;
}

// ── Agents ────────────────────────────────────────────────────────────

export interface ToolInfo {
  id: number;
  name: string;
  description: string;
  language: string;
  code?: string;
  version?: string;
  tool_type?: string;
  domain?: string;
  parameters_json?: string | null;
  allowed_agents_json?: string | null;
  safety_level?: string | null;
}

export interface AgentInfo {
  id: number;
  name: string;
  description: string;
  domain: string;
  tools?: ToolInfo[];
  instructions?: string;
  model?: string;
  version?: string;
  code?: string | null;
  source_files?: string[];
  response_format?: string;
}

// ── Supervisors ───────────────────────────────────────────────────────

export interface Supervisor {
  id: number;
  name: string;
  description: string;
  domain: string;
  preamble?: string;
  model?: string;
  temperature?: number;
  is_active?: boolean;
  agents?: AgentInfo[];
}

// ── Workflows ─────────────────────────────────────────────────────────

export interface Workflow {
  id: number;
  name: string;
  description?: string | null;
  status?: string;
  created_at: string;
  updated_at: string;
}

export interface WorkflowStep {
  id: number;
  workflow_id: number;
  step_order?: number;
  name?: string;
  tool_name?: string | null;
  config_json?: string | null;
}

export interface WorkflowRun {
  id: number;
  workflow_id: number;
  status: string;
  started_at: string;
  finished_at?: string | null;
  result_summary?: string | null;
}

export interface WorkflowStepRun {
  id: number;
  run_id: number;
  step_name: string;
  status: string;
  output_json?: string | null;
  started_at: string;
  finished_at?: string | null;
}

// ── Work Items ────────────────────────────────────────────────────────

export interface WorkItem {
  id: number;
  conversation_id: number;
  title: string;
  status: string;
  surface_kind?: string;
  created_at: string;
  updated_at: string;
}

export interface WorkItemEvent {
  id: number;
  work_item_id: number;
  execution_run_id?: number | null;
  event_type: string;
  status?: string | null;
  summary?: string | null;
  created_at: string;
}

// ── Artifacts ─────────────────────────────────────────────────────────

export interface Artifact {
  id: number;
  conversation_id: number;
  execution_run_id?: number | null;
  kind: string;
  label: string;
  content?: string | null;
  file_path?: string | null;
  content_type?: string | null;
  created_at: string;
}

// ── Planners ──────────────────────────────────────────────────────────

export interface Planner {
  id: number;
  name: string;
  instructions: string;
  model?: string;
  is_active?: boolean;
}

// ── Version Control ───────────────────────────────────────────────────

export interface EntityVersion {
  id: number;
  entity_type: string;
  entity_id: number;
  tag: string;
  created_at: string;
  description?: string | null;
}

// ── Health / System ───────────────────────────────────────────────────

export interface RuntimeHealth {
  status: string;
  version: string;
  uptime_secs: number;
  db_connected: boolean;
  ollama_reachable: boolean;
}

export interface SystemLoad {
  cpu_percent: number;
  memory_percent: number;
  agent_count: number;
  active_conversations: number;
}

export interface RuntimeRegistryStatus {
  last_refresh?: string | null;
  agent_count: number;
  tool_count: number;
}

// ── Supervision ───────────────────────────────────────────────────────

export interface ComputerSupervisionState {
  enabled: boolean;
  allowlist?: string;
  pending_action?: Record<string, unknown> | null;
  recent_executions?: string[];
}

// ── Credentials ───────────────────────────────────────────────────────

export interface CredentialSummary {
  id: number;
  name: string;
  cred_type?: string;
}

// ── Workflow Templates ────────────────────────────────────────────────

export interface WorkflowTemplate {
  id: number;
  name: string;
  description?: string | null;
  category?: string;
}

// ── Browser ───────────────────────────────────────────────────────────

export interface BrowserSessionInfo {
  session?: { id: number };
  exists?: boolean;
}

// ── API Error ─────────────────────────────────────────────────────────

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly body: string,
  ) {
    super(`API error ${status}: ${body}`);
    this.name = 'ApiError';
  }
}
