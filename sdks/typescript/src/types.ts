/** Exact decimal record ID returned by the SDK. */
export type ApiId = string;
/** Accepted input: decimal string, bigint, or a safe integer. */
export type IdInput = string | bigint | number;

/** TypeScript types matching all Overseer REST API types. */

// ── Auth ──────────────────────────────────────────────────────────────

export interface UserProfile {
  id: ApiId;
  username: string;
  display_name: string;
}

export interface AuthResponse {
  token: string;
  user: UserProfile;
}

// ── Conversations ─────────────────────────────────────────────────────

export interface Conversation {
  id: ApiId;
  title: string;
  created_at: string;
}

export interface ConversationSearchResult {
  conversation_id: ApiId;
  conversation_title: string;
  message_id: ApiId;
  content: string;
  created_at: string;
}

// ── Messages & Attachments ────────────────────────────────────────────

export interface Message {
  id: ApiId;
  conversation_id: ApiId;
  role: string;
  content: string;
  created_at: string;
  hide_evidence: boolean;
  attachments: Attachment[];
}

export interface Attachment {
  id: ApiId;
  message_id: ApiId;
  filename: string;
  file_path: string;
  content_type: string;
  size: number;
  extracted_content: string | null;
}

// ── Agents ────────────────────────────────────────────────────────────

export interface ToolInfo {
  id: ApiId;
  name: string;
  description: string;
  code: string;
  language: string;
  script_path: string | null;
  required_intent: string | null;
  safety_level: string;
  version: string;
}

export interface AgentInfo {
  id: ApiId;
  name: string;
  description: string;
  tools: string[];
  instructions: string;
  model: string;
  domain: string;
  version: string;
  code: string | null;
  source_files: string[];
  response_format: string;
}

// ── Supervisors ───────────────────────────────────────────────────────

export interface Supervisor {
  id: ApiId;
  name: string;
  description: string | null;
  instructions: string | null;
  model: string | null;
  domain: string;
  version: string | null;
  code: string | null;
  bypass_synthesizer: boolean;
  fast_path_keywords: string | null;
  verification_agent_id: ApiId | null;
  routing_priority: number;
  hide_evidence: boolean;
  report_style: string | null;
  skip_fast_path_if_artifacts: boolean;
  artifact_fast_path_keywords: string | null;
  response_format: string;
}

// ── Workflows ─────────────────────────────────────────────────────────

export interface Workflow {
  id: ApiId;
  name: string;
  description: string | null;
  trigger_type: string;
  trigger_config: string | null;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface WorkflowStep {
  id: ApiId;
  workflow_id: ApiId;
  step_order: number;
  name: string;
  prompt: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface WorkflowRun {
  id: ApiId;
  workflow_id: ApiId;
  conversation_id: ApiId | null;
  status: string;
  summary: string | null;
  error_message: string | null;
  created_at: string;
  finished_at: string | null;
}

export interface WorkflowStepRun {
  id: ApiId;
  workflow_run_id: ApiId;
  workflow_step_id: ApiId;
  execution_run_id: ApiId | null;
  step_order: number;
  status: string;
  output: string | null;
  error_message: string | null;
  created_at: string;
  finished_at: string | null;
}

// ── Work Items ────────────────────────────────────────────────────────

export interface WorkItem {
  id: ApiId;
  conversation_id: ApiId;
  title: string;
  brief: string | null;
  status: string;
  surface_kind: string;
  assigned_domain: string | null;
  prompt: string;
  source_artifact_ids_json: string | null;
  metadata_json: string | null;
  created_at: string;
  updated_at: string;
}

export interface WorkItemEvent {
  id: ApiId;
  work_item_id: ApiId;
  execution_run_id: ApiId | null;
  event_type: string;
  status: string | null;
  summary: string | null;
  metadata_json: string | null;
  created_at: string;
}

// ── Artifacts ─────────────────────────────────────────────────────────

export interface Artifact {
  id: ApiId;
  conversation_id: ApiId;
  execution_run_id: ApiId | null;
  domain: string | null;
  step_text: string | null;
  kind: string;
  label: string;
  content: string | null;
  file_path: string | null;
  content_type: string | null;
  metadata_json: string | null;
  created_at: string;
}

// ── Planners ──────────────────────────────────────────────────────────

export interface Planner {
  id: ApiId;
  name: string;
  description: string | null;
  instructions: string | null;
  model: string | null;
  domain: string;
  version: string | null;
  code: string | null;
  response_format: string;
}

// ── Version Control ───────────────────────────────────────────────────

export interface EntityVersion {
  id: ApiId;
  version_tag: string;
  changelog: string | null;
  created_at: string;
}

// ── Health / System ───────────────────────────────────────────────────

export interface RuntimeHealth {
  ollama_reachable: boolean;
  ollama_endpoint: string;
  available_models: string[];
  selected_runtime_model: string;
  runtime_model_available: boolean;
  title_model: string;
  title_model_available: boolean;
  python_command: string | null;
  protoc_path: string | null;
  runtime_dir: string;
  native_runtime: boolean;
}

export interface SystemLoad {
  label: string;
  usage: number;
}

export interface RuntimeRegistryStatus {
  status: string;
  refreshed_at: string | null;
  error: string | null;
}

// ── Supervision ───────────────────────────────────────────────────────

export interface ComputerSupervisionState {
  supervision_mode: string;
  session_state: string;
  session_note: string;
  allowlist: string[];
  max_actions: number;
  action_count: number;
  timeout_ms: number;
  approval_state: string;
  pending_tool: string;
  pending_action: string;
  pending_domain: string;
  session_started_at: string | null;
}

// ── Credentials ───────────────────────────────────────────────────────

export interface CredentialSummary {
  id: ApiId;
  name: string;
  cred_type?: string;
}

// ── Workflow Templates ────────────────────────────────────────────────

export interface WorkflowTemplate {
  id: ApiId;
  name: string;
  description?: string | null;
  category?: string;
}

// ── Browser ───────────────────────────────────────────────────────────

export interface BrowserSessionInfo {
  id?: ApiId;
  session?: { id: ApiId };
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

export interface ServiceHealth {
  status: string;
  version: string;
  uptime_secs: number;
  db_connected: boolean;
  ollama_reachable: boolean;
}
