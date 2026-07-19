/**
 * OverseerClient — HTTP client for the Overseer REST API.
 *
 * Usage:
 *   const client = new OverseerClient({ baseUrl: 'http://localhost:8080', token: '...' });
 *   const conversations = await client.conversations.list();
 *   const agents = await client.agents.list();
 */

import ky, { type KyInstance, type Options as KyOptions } from 'ky';

import { AuthManager } from './auth';
import type {
  AgentInfo,
  Artifact,
  Attachment,
  AuthResponse,
  BrowserSessionInfo,
  ComputerSupervisionState,
  Conversation,
  ConversationSearchResult,
  CredentialSummary,
  EntityVersion,
  Message,
  Planner,
  RuntimeHealth,
  RuntimeRegistryStatus,
  Supervisor,
  SystemLoad,
  ToolInfo,
  UserProfile,
  WorkItem,
  WorkItemEvent,
  Workflow,
  WorkflowRun,
  WorkflowStep,
  WorkflowStepRun,
  WorkflowTemplate,
} from './types';
import { ApiError } from './types';

export type { ApiError } from './types';

export interface OverseerClientOptions {
  /** Base URL of the Overseer server (default: http://localhost:8080) */
  baseUrl?: string;
  /** Bearer token for authentication */
  token?: string;
  /** Request timeout in ms (default: 30000) */
  timeout?: number;
  /** Prefix for all request paths */
  prefixUrl?: string;
}

export class OverseerClient {
  private readonly http: KyInstance;
  private readonly auth: AuthManager;

  // Namespaced API modules
  readonly conversations: ConversationsAPI;
  readonly messages: MessagesAPI;
  readonly agents: AgentsAPI;
  readonly tools: ToolsAPI;
  readonly supervisors: SupervisorsAPI;
  readonly workflows: WorkflowsAPI;
  readonly workItems: WorkItemsAPI;
  readonly artifacts: ArtifactsAPI;
  readonly planners: PlannersAPI;
  readonly supervision: SupervisionAPI;
  readonly versions: VersionsAPI;
  readonly browser: BrowserAPI;
  readonly credentials: CredentialsAPI;
  readonly templates: TemplatesAPI;
  readonly settings: SettingsAPI;
  readonly system: SystemAPI;

  constructor(options: OverseerClientOptions = {}) {
    const baseUrl = options.baseUrl ?? 'http://localhost:8080';
    this.auth = new AuthManager();
    if (options.token) this.auth.token = options.token;

    const kyOptions: KyOptions = {
      prefixUrl: options.prefixUrl ?? baseUrl,
      timeout: options.timeout ?? 30_000,
      headers: {
        'User-Agent': 'overseer-client-ts/0.1',
        Accept: 'application/json',
      },
      hooks: {
        beforeRequest: [
          (request) => {
            // Attach auth header
            const headers = this.auth.authHeaders;
            for (const [key, value] of Object.entries(headers)) {
              request.headers.set(key, value);
            }
          },
        ],
        afterResponse: [
          async (_request, _options, response) => {
            if (!response.ok) {
              const body = await response.text().catch(() => '');
              throw new ApiError(response.status, body.slice(0, 500));
            }
          },
        ],
      },
    };

    this.http = ky.create(kyOptions);

    // Initialize API modules
    this.conversations = new ConversationsAPI(this);
    this.messages = new MessagesAPI(this);
    this.agents = new AgentsAPI(this);
    this.tools = new ToolsAPI(this);
    this.supervisors = new SupervisorsAPI(this);
    this.workflows = new WorkflowsAPI(this);
    this.workItems = new WorkItemsAPI(this);
    this.artifacts = new ArtifactsAPI(this);
    this.planners = new PlannersAPI(this);
    this.supervision = new SupervisionAPI(this);
    this.versions = new VersionsAPI(this);
    this.browser = new BrowserAPI(this);
    this.credentials = new CredentialsAPI(this);
    this.templates = new TemplatesAPI(this);
    this.settings = new SettingsAPI(this);
    this.system = new SystemAPI(this);
  }

  /** Current auth token */
  get token(): string | undefined {
    return this.auth.token;
  }

  /** Set auth token */
  set token(value: string | undefined) {
    this.auth.token = value;
  }

  /** Login with username/password */
  async login(username: string, password: string): Promise<string> {
    const data = await this._post<AuthResponse>('/v1/auth/login', { username, password });
    this.auth.setAuth(data);
    return data.token;
  }

  /** Register a new user */
  async register(username: string, display_name: string, password: string): Promise<AuthResponse> {
    const data = await this._post<AuthResponse>('/v1/auth/register', { username, display_name, password });
    this.auth.setAuth(data);
    return data;
  }

  /** Get current user profile */
  async me(): Promise<UserProfile> {
    return this._get<UserProfile>('/v1/auth/me');
  }

  /** Logout */
  async logout(): Promise<void> {
    try {
      await this._post('/v1/auth/logout', {});
    } finally {
      this.auth.clear();
    }
  }

  /** Orchestrate — send query to agent system */
  async orchestrate(conversation_id: number, query: string): Promise<string> {
    const r = await this._rawPost('/api/orchestrate', { conversation_id, query });
    return r.text();
  }

  // ── Low-level HTTP helpers ──────────────────────────────────────────

  async _get<T>(path: string, params?: Record<string, string | number | undefined>): Promise<T> {
    const searchParams = new URLSearchParams();
    if (params) {
      for (const [key, value] of Object.entries(params)) {
        if (value !== undefined) searchParams.set(key, String(value));
      }
    }
    return this.http.get(path, { searchParams }).json<T>();
  }

  async _post<T = unknown>(path: string, body?: unknown): Promise<T> {
    return this.http.post(path, { json: body ?? {} }).json<T>();
  }

  async _patch(path: string, body?: unknown): Promise<void> {
    await this.http.patch(path, { json: body ?? {} });
  }

  async _delete(path: string): Promise<void> {
    await this.http.delete(path);
  }

  async _rawPost(path: string, body?: unknown): Promise<Response> {
    return this.http.post(path, { json: body ?? {} });
  }

  async _getList<T>(path: string, params?: Record<string, string | number | undefined>): Promise<T[]> {
    return this._get<T[]>(path, params);
  }
}

// ── Auth (top-level methods are on client) ───────────────────────────

// ── Conversations ─────────────────────────────────────────────────────

export class ConversationsAPI {
  constructor(private readonly client: OverseerClient) {}

  async list(limit = 50): Promise<Conversation[]> {
    return this.client._getList<Conversation>('/api/conversations', { limit });
  }

  async create(title: string): Promise<number> {
    return this.client._post<number>('/api/conversations', { title });
  }

  async updateTitle(id: number, title: string): Promise<void> {
    await this.client._post('/api/conversations/update_title', { id, title });
  }

  async delete(id: number): Promise<void> {
    await this.client._post('/api/conversations/delete', { id });
  }

  async search(query: string): Promise<ConversationSearchResult[]> {
    return this.client._getList<ConversationSearchResult>('/api/conversations/search', { q: query });
  }
}

// ── Messages ──────────────────────────────────────────────────────────

export class MessagesAPI {
  constructor(private readonly client: OverseerClient) {}

  async list(conversationId: number): Promise<Message[]> {
    return this.client._getList<Message>('/api/messages', { conversation_id: conversationId });
  }

  async send(conversationId: number, role: string, content: string, hideEvidence = false): Promise<number> {
    return this.client._post<number>('/api/messages', {
      conversation_id: conversationId, role, content, hide_evidence: hideEvidence,
    });
  }

  async delete(messageId: number): Promise<void> {
    await this.client._post('/api/messages/delete', { id: messageId });
  }

  async listAttachments(messageId: number): Promise<Attachment[]> {
    return this.client._getList<Attachment>('/api/attachments', { message_id: messageId });
  }

  async addAttachment(messageId: number, filename: string, filePath: string, contentType: string, size: number): Promise<number> {
    return this.client._post<number>('/api/attachments', {
      message_id: messageId, filename, file_path: filePath, content_type: contentType, size,
    });
  }
}

// ── Agents ────────────────────────────────────────────────────────────

export class AgentsAPI {
  constructor(private readonly client: OverseerClient) {}

  async list(): Promise<AgentInfo[]> {
    return this.client._getList<AgentInfo>('/api/agents');
  }

  async listAll(): Promise<AgentInfo[]> {
    return this.client._getList<AgentInfo>('/api/agents/all');
  }

  async create(name: string, description: string, domain: string): Promise<number> {
    return this.client._post<number>('/api/agents', { name, description, domain });
  }

  async update(agent: AgentInfo): Promise<void> {
    await this.client._post('/api/agents/update', agent);
  }

  async getByDomain(domain: string): Promise<AgentInfo | null> {
    return this.client._get<AgentInfo | null>('/api/agents/by_domain', { domain });
  }
}

// ── Tools ─────────────────────────────────────────────────────────────

export class ToolsAPI {
  constructor(private readonly client: OverseerClient) {}

  async list(): Promise<ToolInfo[]> {
    return this.client._getList<ToolInfo>('/api/tools');
  }

  async create(name: string, description: string, language: string): Promise<number> {
    return this.client._post<number>('/api/tools', { name, description, language });
  }

  async update(tool: ToolInfo): Promise<void> {
    await this.client._post('/api/tools/update', tool);
  }

  async linkToAgent(agentId: number, toolId: number): Promise<void> {
    await this.client._post('/api/tools/link', { agent_id: agentId, tool_id: toolId });
  }

  async unlinkFromAgent(agentId: number, toolId: number): Promise<void> {
    await this.client._post('/api/tools/unlink', { agent_id: agentId, tool_id: toolId });
  }
}

// ── Supervisors ───────────────────────────────────────────────────────

export class SupervisorsAPI {
  constructor(private readonly client: OverseerClient) {}

  async list(): Promise<Supervisor[]> {
    return this.client._getList<Supervisor>('/api/supervisors');
  }

  async create(name: string, description: string, domain: string): Promise<number> {
    return this.client._post<number>('/api/supervisors', { name, description, domain });
  }

  async update(supervisor: Supervisor): Promise<void> {
    await this.client._post('/api/supervisors/update', supervisor);
  }

  async delete(id: number): Promise<void> {
    await this.client._post('/api/supervisors/delete', { id });
  }

  async getTools(supervisorId: number): Promise<ToolInfo[]> {
    return this.client._getList<ToolInfo>(`/api/supervisors/${supervisorId}/tools`);
  }

  async getAgents(supervisorId: number): Promise<AgentInfo[]> {
    return this.client._getList<AgentInfo>(`/api/supervisors/${supervisorId}/agents`);
  }

  async linkAgent(supervisorId: number, agentId: number): Promise<void> {
    await this.client._post('/api/supervisors/link', { supervisor_id: supervisorId, agent_id: agentId });
  }

  async unlinkAgent(supervisorId: number, agentId: number): Promise<void> {
    await this.client._post('/api/supervisors/unlink', { supervisor_id: supervisorId, agent_id: agentId });
  }
}

// ── Workflows ─────────────────────────────────────────────────────────

export class WorkflowsAPI {
  constructor(private readonly client: OverseerClient) {}

  async list(): Promise<Workflow[]> {
    return this.client._getList<Workflow>('/api/workflows');
  }

  async create(name: string): Promise<number> {
    return this.client._post<number>('/api/workflows', { name });
  }

  async update(workflow: Workflow): Promise<void> {
    await this.client._post('/api/workflows/update', workflow);
  }

  async delete(id: number): Promise<void> {
    await this.client._post('/api/workflows/delete', { id });
  }

  async getSteps(workflowId: number): Promise<WorkflowStep[]> {
    return this.client._getList<WorkflowStep>(`/api/workflows/${workflowId}/steps`);
  }

  async addStep(workflowId: number): Promise<number> {
    return this.client._post<number>(`/api/workflows/${workflowId}/steps`, {});
  }

  async updateStep(step: WorkflowStep): Promise<void> {
    await this.client._post('/api/workflows/steps/update', step);
  }

  async deleteStep(stepId: number): Promise<void> {
    await this.client._post('/api/workflows/steps/delete', { id: stepId });
  }

  async getRuns(workflowId: number): Promise<WorkflowRun[]> {
    return this.client._getList<WorkflowRun>(`/api/workflows/${workflowId}/runs`);
  }

  async getStepRuns(runId: number): Promise<WorkflowStepRun[]> {
    return this.client._getList<WorkflowStepRun>(`/api/workflows/runs/${runId}/steps`);
  }

  async run(workflowId: number): Promise<number> {
    return this.client._post<number>(`/api/workflows/${workflowId}/run`, {});
  }

  async publish(workflowId: number): Promise<void> {
    await this.client._post(`/v1/workflows/${workflowId}/publish`, {});
  }

  async getVersions(workflowId: number): Promise<Record<string, unknown>[]> {
    return this.client._getList<Record<string, unknown>>(`/v1/workflows/${workflowId}/versions`);
  }

  async restoreVersion(workflowId: number, version: number): Promise<void> {
    await this.client._post(`/v1/workflows/${workflowId}/versions/${version}/restore`, {});
  }
}

// ── Work Items ────────────────────────────────────────────────────────

export class WorkItemsAPI {
  constructor(private readonly client: OverseerClient) {}

  async list(conversationId: number): Promise<WorkItem[]> {
    return this.client._getList<WorkItem>('/api/work_items', { conversation_id: conversationId });
  }

  async create(params: Record<string, unknown>): Promise<number> {
    return this.client._post<number>('/api/work_items', params);
  }

  async updateStatus(id: number, status: string): Promise<void> {
    await this.client._post('/api/work_items/update_status', { id, status });
  }

  async logEvent(params: Record<string, unknown>): Promise<number> {
    return this.client._post<number>('/api/work_items/event', params);
  }

  async getEvents(workItemId: number, limit = 50): Promise<WorkItemEvent[]> {
    return this.client._getList<WorkItemEvent>(`/api/work_items/${workItemId}/events`, { limit });
  }
}

// ── Artifacts ─────────────────────────────────────────────────────────

export class ArtifactsAPI {
  constructor(private readonly client: OverseerClient) {}

  async list(conversationId: number): Promise<Artifact[]> {
    return this.client._getList<Artifact>('/api/artifacts', { conversation_id: conversationId });
  }

  async create(params: Record<string, unknown>): Promise<number> {
    return this.client._post<number>('/api/artifacts', params);
  }

  async getByIds(ids: number[]): Promise<Artifact[]> {
    return this.client._post<Artifact[]>('/api/artifacts/by_ids', ids);
  }

  async getByRun(executionRunId: number): Promise<Artifact[]> {
    return this.client._getList<Artifact>('/api/artifacts/by_run', { execution_run_id: executionRunId });
  }
}

// ── Planners ──────────────────────────────────────────────────────────

export class PlannersAPI {
  constructor(private readonly client: OverseerClient) {}

  async list(): Promise<Planner[]> {
    return this.client._getList<Planner>('/api/planners');
  }

  async create(params: Record<string, unknown>): Promise<number> {
    return this.client._post<number>('/api/planners', params);
  }
}

// ── Supervision ───────────────────────────────────────────────────────

export class SupervisionAPI {
  constructor(private readonly client: OverseerClient) {}

  async getState(): Promise<ComputerSupervisionState> {
    return this.client._get<ComputerSupervisionState>('/api/supervision/state');
  }

  async approveAction(): Promise<void> {
    await this.client._post('/api/supervision/approve', {});
  }

  async rejectAction(): Promise<void> {
    await this.client._post('/api/supervision/reject', {});
  }

  async activate(enabled: boolean, allowlist = ''): Promise<void> {
    await this.client._post('/api/supervision/activate', { enabled, allowlist });
  }

  async clearPending(): Promise<void> {
    await this.client._post('/api/supervision/clear_pending', {});
  }

  async listPendingApprovals(): Promise<Record<string, unknown>[]> {
    return this.client._getList<Record<string, unknown>>('/api/kernel/approvals');
  }

  async kernelApprove(approvalId: string): Promise<Record<string, unknown>> {
    return this.client._post<Record<string, unknown>>('/api/kernel/approve', { approval_id: approvalId, resolver: 'api' });
  }

  async kernelReject(approvalId: string): Promise<Record<string, unknown>> {
    return this.client._post<Record<string, unknown>>('/api/kernel/reject', { approval_id: approvalId, resolver: 'api' });
  }

  async kernelEmergencyStop(runId?: number): Promise<Record<string, unknown>> {
    return this.client._post<Record<string, unknown>>('/api/kernel/emergency_stop', { run_id: runId ?? null });
  }
}

// ── Version Control ───────────────────────────────────────────────────

export class VersionsAPI {
  constructor(private readonly client: OverseerClient) {}

  async publish(params: Record<string, unknown>): Promise<void> {
    await this.client._post('/api/version/publish', params);
  }

  async publishAgent(agentId: number, tag: string, description = ''): Promise<void> {
    await this.client._post('/api/version/publish_agent', { agent_id: agentId, tag, description });
  }

  async publishTool(toolId: number, tag: string, description = ''): Promise<void> {
    await this.client._post('/api/version/publish_tool', { tool_id: toolId, tag, description });
  }

  async rollback(params: Record<string, unknown>): Promise<void> {
    await this.client._post('/api/version/rollback', params);
  }

  async getContent(entityType: string, entityId: number, tag: string): Promise<string> {
    return this.client._get<string>('/api/version/content', { entity_type: entityType, entity_id: entityId, tag });
  }

  async getHistory(entityType: string, entityId: number): Promise<EntityVersion[]> {
    return this.client._getList<EntityVersion>('/api/version/history', { entity_type: entityType, entity_id: entityId });
  }
}

// ── Browser ───────────────────────────────────────────────────────────

export class BrowserAPI {
  constructor(private readonly client: OverseerClient) {}

  async createSession(): Promise<Record<string, unknown>> {
    return this.client._post<Record<string, unknown>>('/v1/browser/sessions', {});
  }

  async getSession(sessionId: number): Promise<BrowserSessionInfo> {
    return this.client._get<BrowserSessionInfo>(`/v1/browser/sessions/${sessionId}`);
  }

  async closeSession(sessionId: number): Promise<void> {
    await this.client._delete(`/v1/browser/sessions/${sessionId}`);
  }

  async navigate(sessionId: number, url: string): Promise<Record<string, unknown>> {
    return this.client._post<Record<string, unknown>>(`/v1/browser/sessions/${sessionId}/navigate`, { url });
  }

  async screenshot(sessionId: number): Promise<Record<string, unknown>> {
    return this.client._get<Record<string, unknown>>(`/v1/browser/sessions/${sessionId}/screenshot`);
  }

  async setPolicy(sessionId: number, policy: Record<string, unknown>): Promise<void> {
    await this.client._post(`/v1/browser/sessions/${sessionId}/policy`, policy);
  }

  async getAuditLog(sessionId: number): Promise<Record<string, unknown>[]> {
    return this.client._getList<Record<string, unknown>>(`/v1/browser/sessions/${sessionId}/audit`);
  }

  async memoryReport(): Promise<Record<string, unknown>> {
    return this.client._get<Record<string, unknown>>('/v1/browser/memory');
  }

  async startRecording(sessionId: number, quality = 75): Promise<Record<string, unknown>> {
    return this.client._post<Record<string, unknown>>(`/v1/browser/sessions/${sessionId}/recording/start`, { quality });
  }

  async stopRecording(sessionId: number): Promise<Record<string, unknown>> {
    return this.client._post<Record<string, unknown>>(`/v1/browser/sessions/${sessionId}/recording/stop`, {});
  }

  async getRecording(sessionId: number): Promise<Record<string, unknown>> {
    return this.client._get<Record<string, unknown>>(`/v1/browser/sessions/${sessionId}/recording`);
  }

  async getRecordingEvents(sessionId: number): Promise<Record<string, unknown>[]> {
    return this.client._getList<Record<string, unknown>>(`/v1/browser/sessions/${sessionId}/recording/events`);
  }

  async resumeHITL(sessionId: number): Promise<void> {
    await this.client._post(`/v1/browser/sessions/${sessionId}/resume`, {});
  }
}

// ── Credentials ───────────────────────────────────────────────────────

export class CredentialsAPI {
  constructor(private readonly client: OverseerClient) {}

  async list(): Promise<CredentialSummary[]> {
    return this.client._getList<CredentialSummary>('/v1/credentials');
  }

  async create(name: string, value: string, credType = 'api_key'): Promise<CredentialSummary> {
    return this.client._post<CredentialSummary>('/v1/credentials', { name, value, cred_type: credType });
  }

  async update(id: number, params: { name?: string; value?: string }): Promise<void> {
    await this.client._patch(`/v1/credentials/${id}`, params);
  }

  async delete(id: number): Promise<void> {
    await this.client._delete(`/v1/credentials/${id}`);
  }
}

// ── Workflow Templates ───────────────────────────────────────────────

export class TemplatesAPI {
  constructor(private readonly client: OverseerClient) {}

  async list(): Promise<WorkflowTemplate[]> {
    return this.client._getList<WorkflowTemplate>('/v1/workflow-templates');
  }

  async create(params: Record<string, unknown>): Promise<WorkflowTemplate> {
    return this.client._post<WorkflowTemplate>('/v1/workflow-templates', params);
  }

  async install(templateId: number): Promise<number> {
    return this.client._post<number>(`/v1/workflow-templates/${templateId}/install`, {});
  }
}

// ── Settings ──────────────────────────────────────────────────────────

export class SettingsAPI {
  constructor(private readonly client: OverseerClient) {}

  async get(key: string): Promise<string | null> {
    return this.client._get<string | null>('/api/settings', { key });
  }

  async set(key: string, value: string): Promise<void> {
    await this.client._post('/api/settings', { key, value });
  }

  async getAll(): Promise<Record<string, string>> {
    return this.client._get<Record<string, string>>('/api/settings/all');
  }
}

// ── System ────────────────────────────────────────────────────────────

export class SystemAPI {
  constructor(private readonly client: OverseerClient) {}

  async health(): Promise<RuntimeHealth> {
    return this.client._get<RuntimeHealth>('/health');
  }

  async load(): Promise<SystemLoad> {
    return this.client._get<SystemLoad>('/api/system/load');
  }

  async models(): Promise<string[]> {
    return this.client._getList<string>('/api/models/available');
  }

  async registryStatus(): Promise<RuntimeRegistryStatus> {
    return this.client._get<RuntimeRegistryStatus>('/api/registry/status');
  }

  async refreshRegistry(): Promise<void> {
    await this.client._post('/api/registry/refresh', {});
  }

  async exportFile(filename: string, content: string): Promise<string> {
    return this.client._post<string>('/api/system/export', { filename, content });
  }
}
