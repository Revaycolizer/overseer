/**
 * OverseerClient — HTTP client for the Overseer REST API.
 * Zero dependencies — uses native fetch (Node.js 18+ / modern browsers).
 *
 * Usage:
 *   const client = new OverseerClient({ baseUrl: 'http://localhost:8080', token: '...' });
 *   const conversations = await client.conversations.list();
 *   const agents = await client.agents.list();
 */

import { ChatAPI } from './chat.js';
import { decodeJson, encodeJson, idToDecimal } from './json.js';
import type { ApiId, IdInput } from './types.js';
import { AuthManager } from './auth.js';
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
  ServiceHealth,
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
} from './types.js';
import { ApiError } from './types.js';

export type { ApiError } from './types.js';

export interface OverseerClientOptions {
  /** Base URL of the Overseer server (default: http://localhost:8080) */
  baseUrl?: string;
  /** Bearer token for authentication */
  token?: string;
  /** Request timeout in ms (default: 30000) */
  timeout?: number;
  /** Custom fetch implementation for extensions and tests. */
  fetch?: FetchFn;
  /** WebSocket factory for runtimes without native WebSocket. */
  webSocketFactory?: (url: string) => WebSocket;
}

/**
 * Default fetch implementation — uses global fetch (Node.js 18+ / browser).
 * Override via constructor for custom HTTP logic.
 */
export type FetchFn = typeof fetch;

export class OverseerClient {
  private readonly baseUrl: string;
  private readonly timeout: number;
  private readonly auth: AuthManager;
  private readonly _fetch: FetchFn;

  // Namespaced API modules
  readonly chat: ChatAPI;
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
    this.baseUrl = (options.baseUrl ?? 'http://localhost:8080').replace(/\/+$/, '');
    const parsedUrl = new URL(this.baseUrl);
    if (!['http:', 'https:'].includes(parsedUrl.protocol) || parsedUrl.username || parsedUrl.password || parsedUrl.pathname !== '/' || parsedUrl.search || parsedUrl.hash) throw new TypeError('baseUrl must be an HTTP(S) server origin without credentials');
    this.timeout = options.timeout ?? 30_000;
    if (!Number.isFinite(this.timeout) || this.timeout <= 0) throw new RangeError('timeout must be positive');
    this.auth = new AuthManager();
    if (options.token) this.auth.token = options.token;
    this._fetch = options.fetch ?? (typeof fetch !== 'undefined' ? fetch.bind(globalThis) : (() => { throw new Error('fetch is not available — use Node.js 18+ or a browser') }) as FetchFn);

    this.chat = new ChatAPI(this.baseUrl, () => this.token, options.webSocketFactory);
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
  get token(): string | undefined { return this.auth.token; }
  set token(value: string | undefined) { this.auth.token = value; }

  /** Login and store token */
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
  async me(): Promise<UserProfile> { return this._get<UserProfile>('/v1/auth/me'); }

  /** Logout */
  async logout(): Promise<void> {
    try { await this._post('/v1/auth/logout', {}); } finally { this.auth.clear(); }
  }

  /** Orchestrate — send query to agent system */
  async orchestrate(conversation_id: IdInput, query: string): Promise<string> {
    const r = await this._raw('POST', '/api/orchestrate', { conversation_id, query });
    return r.text();
  }

  /** Call an additional JSON endpoint on the configured server. */
  async request<T = unknown>(method: string, path: string, body?: unknown): Promise<T> {
    return this._requestJson(method, path, body) as Promise<T>;
  }

  // ── Low-level HTTP ─────────────────────────────────────────────────

  async _get<T>(path: string, params?: Record<string, string | number | bigint | undefined>): Promise<T> {
    const url = this._buildUrl(path, params);
    const r = await this._raw('GET', url);
    return decodeJson(await r.text()) as T;
  }

  async _getList<T>(path: string, params?: Record<string, string | number | bigint | undefined>): Promise<T[]> {
    return this._get<T[]>(path, params);
  }

  async _post<T = unknown>(path: string, body?: unknown): Promise<T> {
    return this._requestJson('POST', path, body) as Promise<T>;
  }

  async _patch(path: string, body?: unknown): Promise<void> {
    await this._requestJson('PATCH', path, body);
  }

  async _delete(path: string): Promise<void> {
    const r = await this._raw('DELETE', path);
    if (!r.ok && r.status !== 204) await this._handleError(r);
  }

  private async _requestJson(method: string, path: string, body?: unknown): Promise<unknown> {
    const r = await this._raw(method, path, body);
    const text = await r.text();
    if (!text) return undefined;
    return decodeJson(text);
  }

  private _buildUrl(path: string, params?: Record<string, string | number | bigint | undefined>): string {
    const url = new URL(path, this.baseUrl);
    if (params) {
      for (const [key, value] of Object.entries(params)) {
        if (value !== undefined) url.searchParams.set(key, (key === "id" || key.endsWith("_id")) ? idToDecimal(value) : String(value));
      }
    }
    return url.toString();
  }

  private async _raw(method: string, url: string, body?: unknown): Promise<Response> {
    const headers: Record<string, string> = {
      'User-Agent': 'overseer-client-ts/0.2.0',
      'Accept': 'application/json',
      ...this.auth.authHeaders,
    };
    const target = new URL(url, this.baseUrl);
    if (target.origin !== new URL(this.baseUrl).origin) throw new TypeError('Requests must stay on the configured server');
    const init: RequestInit = { method, headers, redirect: 'error' };
    if (body !== undefined) {
      headers['Content-Type'] = 'application/json';
      init.body = encodeJson(body, new URL(url, this.baseUrl).pathname === '/api/artifacts/by_ids');
    }
    const controller = new AbortController();
    init.signal = controller.signal;
    const timer = setTimeout(() => controller.abort(), this.timeout);

    try {
      const r = await this._fetch(target.toString(), init);
      if (!r.ok) await this._handleError(r);
      // Keep the timeout active while the body is arriving, not just through headers.
      const bytes = await r.arrayBuffer();
      return new Response([204, 205, 304].includes(r.status) ? null : bytes, { status: r.status, statusText: r.statusText, headers: r.headers });
    } finally {
      clearTimeout(timer);
    }
  }

  private async _handleError(r: Response): Promise<never> {
    const body = await r.text().catch(() => '');
    throw new ApiError(r.status, body.slice(0, 500));
  }
}

// ── API Modules ──────────────────────────────────────────────────────

export class ConversationsAPI {
  constructor(private readonly c: OverseerClient) {}
  async list(limit = 50): Promise<Conversation[]> { return this.c._getList('/api/conversations', { limit }); }
  async create(title: string): Promise<ApiId> { return this.c._post('/api/conversations', { title }); }
  async updateTitle(id: IdInput, title: string): Promise<void> { await this.c._post('/api/conversations/update_title', { id, title }); }
  async delete(id: IdInput): Promise<void> { await this.c._post('/api/conversations/delete', { id }); }
  async search(query: string): Promise<ConversationSearchResult[]> { return this.c._getList('/api/conversations/search', { q: query }); }
}

export class MessagesAPI {
  constructor(private readonly c: OverseerClient) {}
  async list(conversationId: IdInput): Promise<Message[]> { return this.c._getList('/api/messages', { conversation_id: conversationId }); }
  async send(conversationId: IdInput, role: string, content: string, hideEvidence = false): Promise<ApiId> {
    return this.c._post('/api/messages', { conversation_id: conversationId, role, content, hide_evidence: hideEvidence });
  }
  async delete(messageId: IdInput): Promise<void> { await this.c._post('/api/messages/delete', { id: messageId }); }
  async listAttachments(messageId: IdInput): Promise<Attachment[]> { return this.c._getList('/api/attachments', { message_id: messageId }); }
  async addAttachment(messageId: IdInput, filename: string, filePath: string, contentType: string, size: number): Promise<ApiId> {
    return this.c._post('/api/attachments', { message_id: messageId, filename, file_path: filePath, content_type: contentType, size });
  }
}

export class AgentsAPI {
  constructor(private readonly c: OverseerClient) {}
  async list(): Promise<AgentInfo[]> { return this.c._getList('/api/agents'); }
  async listAll(): Promise<AgentInfo[]> { return this.c._getList('/api/agents/all'); }
  async create(name: string, description: string, domain: string): Promise<ApiId> { return this.c._post('/api/agents', { name, description, domain }); }
  async update(agent: AgentInfo): Promise<void> { await this.c._post('/api/agents/update', agent as never); }
  async getByDomain(domain: string): Promise<AgentInfo | null> { return this.c._get('/api/agents/by_domain', { domain }); }
}

export class ToolsAPI {
  constructor(private readonly c: OverseerClient) {}
  async list(): Promise<ToolInfo[]> { return this.c._getList('/api/tools'); }
  async create(name: string, description: string, language: string): Promise<ApiId> { return this.c._post('/api/tools', { name, description, language }); }
  async update(tool: ToolInfo): Promise<void> { await this.c._post('/api/tools/update', tool as never); }
  async linkToAgent(agentId: IdInput, toolId: IdInput): Promise<void> { await this.c._post('/api/tools/link', { agent_id: agentId, tool_id: toolId }); }
  async unlinkFromAgent(agentId: IdInput, toolId: IdInput): Promise<void> { await this.c._post('/api/tools/unlink', { agent_id: agentId, tool_id: toolId }); }
}

export class SupervisorsAPI {
  constructor(private readonly c: OverseerClient) {}
  async list(): Promise<Supervisor[]> { return this.c._getList('/api/supervisors'); }
  async create(name: string, description: string, domain: string): Promise<ApiId> { return this.c._post('/api/supervisors', { name, description, domain }); }
  async update(supervisor: Supervisor): Promise<void> { await this.c._post('/api/supervisors/update', supervisor as never); }
  async delete(id: IdInput): Promise<void> { await this.c._post('/api/supervisors/delete', { id }); }
  async getTools(supervisorId: IdInput): Promise<ToolInfo[]> { return this.c._getList(`/api/supervisors/${idToDecimal(supervisorId)}/tools`); }
  async getAgents(supervisorId: IdInput): Promise<AgentInfo[]> { return this.c._getList(`/api/supervisors/${idToDecimal(supervisorId)}/agents`); }
  async linkAgent(supervisorId: IdInput, agentId: IdInput): Promise<void> { await this.c._post('/api/supervisors/link', { supervisor_id: supervisorId, agent_id: agentId }); }
  async unlinkAgent(supervisorId: IdInput, agentId: IdInput): Promise<void> { await this.c._post('/api/supervisors/unlink', { supervisor_id: supervisorId, agent_id: agentId }); }
}

export class WorkflowsAPI {
  constructor(private readonly c: OverseerClient) {}
  async list(): Promise<Workflow[]> { return this.c._getList('/api/workflows'); }
  async create(name: string): Promise<ApiId> { return this.c._post('/api/workflows', { name }); }
  async update(workflow: Workflow): Promise<void> { await this.c._post('/api/workflows/update', workflow as never); }
  async delete(id: IdInput): Promise<void> { await this.c._post('/api/workflows/delete', { id }); }
  async getSteps(workflowId: IdInput): Promise<WorkflowStep[]> { return this.c._getList(`/api/workflows/${idToDecimal(workflowId)}/steps`); }
  async addStep(workflowId: IdInput): Promise<ApiId> { return this.c._post(`/api/workflows/${idToDecimal(workflowId)}/steps`, {}); }
  async updateStep(step: WorkflowStep): Promise<void> { await this.c._post('/api/workflows/steps/update', step as never); }
  async deleteStep(stepId: IdInput): Promise<void> { await this.c._post('/api/workflows/steps/delete', { id: stepId }); }
  async getRuns(workflowId: IdInput): Promise<WorkflowRun[]> { return this.c._getList(`/api/workflows/${idToDecimal(workflowId)}/runs`); }
  async getStepRuns(runId: IdInput): Promise<WorkflowStepRun[]> { return this.c._getList(`/api/workflows/runs/${idToDecimal(runId)}/steps`); }
  async run(workflowId: IdInput): Promise<ApiId> { return this.c._post(`/api/workflows/${idToDecimal(workflowId)}/run`, {}); }
  async publish(workflowId: IdInput): Promise<void> { await this.c._post(`/v1/workflows/${idToDecimal(workflowId)}/publish`, {}); }
  async getVersions(workflowId: IdInput): Promise<Record<string, unknown>[]> { return this.c._getList(`/v1/workflows/${idToDecimal(workflowId)}/versions`); }
  async restoreVersion(workflowId: IdInput, version: number): Promise<void> { await this.c._post(`/v1/workflows/${idToDecimal(workflowId)}/versions/${version}/restore`, {}); }
}

export class WorkItemsAPI {
  constructor(private readonly c: OverseerClient) {}
  async list(conversationId: IdInput): Promise<WorkItem[]> { return this.c._getList('/api/work_items', { conversation_id: conversationId }); }
  async create(params: Record<string, unknown>): Promise<ApiId> { return this.c._post('/api/work_items', params); }
  async updateStatus(id: IdInput, status: string): Promise<void> { await this.c._post('/api/work_items/update_status', { id, status }); }
  async logEvent(params: Record<string, unknown>): Promise<ApiId> { return this.c._post('/api/work_items/event', params); }
  async getEvents(workItemId: IdInput, limit = 50): Promise<WorkItemEvent[]> { return this.c._getList(`/api/work_items/${idToDecimal(workItemId)}/events`, { limit }); }
}

export class ArtifactsAPI {
  constructor(private readonly c: OverseerClient) {}
  async list(conversationId: IdInput): Promise<Artifact[]> { return this.c._getList('/api/artifacts', { conversation_id: conversationId }); }
  async create(params: Record<string, unknown>): Promise<ApiId> { return this.c._post('/api/artifacts', params); }
  async getByIds(ids: IdInput[]): Promise<Artifact[]> { return this.c._post('/api/artifacts/by_ids', ids.map(idToDecimal)); }
  async getByRun(executionRunId: IdInput): Promise<Artifact[]> { return this.c._getList('/api/artifacts/by_run', { execution_run_id: executionRunId }); }
}

export class PlannersAPI {
  constructor(private readonly c: OverseerClient) {}
  async list(): Promise<Planner[]> { return this.c._getList('/api/planners'); }
  async create(params: Record<string, unknown>): Promise<ApiId> { return this.c._post('/api/planners', params); }
}

export class SupervisionAPI {
  constructor(private readonly c: OverseerClient) {}
  async getState(): Promise<ComputerSupervisionState> { return this.c._get('/api/supervision/state'); }
  async approveAction(): Promise<void> { await this.c._post('/api/supervision/approve', {}); }
  async rejectAction(): Promise<void> { await this.c._post('/api/supervision/reject', {}); }
  async activate(enabled: boolean, allowlist = ''): Promise<void> { await this.c._post('/api/supervision/activate', { enabled, allowlist }); }
  async clearPending(): Promise<void> { await this.c._post('/api/supervision/clear_pending', {}); }
  async listPendingApprovals(): Promise<Record<string, unknown>[]> { return this.c._getList('/api/kernel/approvals'); }
  async kernelApprove(approvalId: string): Promise<Record<string, unknown>> { return this.c._post('/api/kernel/approve', { approval_id: approvalId, resolver: 'api' }); }
  async kernelReject(approvalId: string): Promise<Record<string, unknown>> { return this.c._post('/api/kernel/reject', { approval_id: approvalId, resolver: 'api' }); }
  async kernelEmergencyStop(runId?: IdInput): Promise<Record<string, unknown>> { return this.c._post('/api/kernel/emergency_stop', { run_id: runId ?? null }); }
}

export class VersionsAPI {
  constructor(private readonly c: OverseerClient) {}
  async publish(params: Record<string, unknown>): Promise<void> { await this.c._post('/api/version/publish', params); }
  async publishAgent(agentId: IdInput, tag: string, description = ''): Promise<void> { await this.c._post('/api/version/publish_agent', { agent_id: agentId, tag, description }); }
  async publishTool(toolId: IdInput, tag: string, description = ''): Promise<void> { await this.c._post('/api/version/publish_tool', { tool_id: toolId, tag, description }); }
  async rollback(params: Record<string, unknown>): Promise<void> { await this.c._post('/api/version/rollback', params); }
  async getContent(entityType: string, entityId: IdInput, tag: string): Promise<string | null> {
    const r = await this.c._get<string | null>(`/api/version/content`, { entity_type: entityType, entity_id: entityId, tag });
    return r;
  }
  async getHistory(entityType: string, entityId: IdInput): Promise<EntityVersion[]> { return this.c._getList('/api/version/history', { entity_type: entityType, entity_id: entityId }); }
}

export class BrowserAPI {
  constructor(private readonly c: OverseerClient) {}
  async createSession(): Promise<Record<string, unknown>> { return this.c._post('/v1/browser/sessions', {}); }
  async getSession(sessionId: IdInput): Promise<BrowserSessionInfo> { return this.c._get(`/v1/browser/sessions/${idToDecimal(sessionId)}`); }
  async closeSession(sessionId: IdInput): Promise<void> { await this.c._delete(`/v1/browser/sessions/${idToDecimal(sessionId)}`); }
  async navigate(sessionId: IdInput, url: string): Promise<Record<string, unknown>> { return this.c._post(`/v1/browser/sessions/${idToDecimal(sessionId)}/navigate`, { url }); }
  async screenshot(sessionId: IdInput): Promise<Record<string, unknown>> { return this.c._get(`/v1/browser/sessions/${idToDecimal(sessionId)}/screenshot`); }
  async setPolicy(sessionId: IdInput, policy: Record<string, unknown>): Promise<void> { await this.c._post(`/v1/browser/sessions/${idToDecimal(sessionId)}/policy`, policy); }
  async getAuditLog(sessionId: IdInput): Promise<Record<string, unknown>[]> { return this.c._getList(`/v1/browser/sessions/${idToDecimal(sessionId)}/audit`); }
  async memoryReport(): Promise<Record<string, unknown>> { return this.c._get('/v1/browser/memory'); }
  async startRecording(sessionId: IdInput, quality = 75): Promise<Record<string, unknown>> { return this.c._post(`/v1/browser/sessions/${idToDecimal(sessionId)}/recording/start`, { quality }); }
  async stopRecording(sessionId: IdInput): Promise<Record<string, unknown>> { return this.c._post(`/v1/browser/sessions/${idToDecimal(sessionId)}/recording/stop`, {}); }
  async getRecording(sessionId: IdInput): Promise<Record<string, unknown>> { return this.c._get(`/v1/browser/sessions/${idToDecimal(sessionId)}/recording`); }
  async getRecordingEvents(sessionId: IdInput): Promise<Record<string, unknown>[]> { return this.c._getList(`/v1/browser/sessions/${idToDecimal(sessionId)}/recording/events`); }
  async resumeHITL(sessionId: IdInput): Promise<void> { await this.c._post(`/v1/browser/sessions/${idToDecimal(sessionId)}/resume`, {}); }
}

export class CredentialsAPI {
  constructor(private readonly c: OverseerClient) {}
  async list(): Promise<CredentialSummary[]> { return this.c._getList('/v1/credentials'); }
  async create(name: string, value: string, credType = 'api_key'): Promise<CredentialSummary> { return this.c._post('/v1/credentials', { name, value, cred_type: credType }); }
  async update(id: IdInput, params: { name?: string; value?: string }): Promise<void> { await this.c._patch(`/v1/credentials/${idToDecimal(id)}`, params); }
  async delete(id: IdInput): Promise<void> { await this.c._delete(`/v1/credentials/${idToDecimal(id)}`); }
}

export class TemplatesAPI {
  constructor(private readonly c: OverseerClient) {}
  async list(): Promise<WorkflowTemplate[]> { return this.c._getList('/v1/workflow-templates'); }
  async create(params: Record<string, unknown>): Promise<WorkflowTemplate> { return this.c._post('/v1/workflow-templates', params); }
  async install(templateId: IdInput): Promise<ApiId> { return this.c._post(`/v1/workflow-templates/${idToDecimal(templateId)}/install`, {}); }
}

export class SettingsAPI {
  constructor(private readonly c: OverseerClient) {}
  async get(key: string): Promise<string | null> { return this.c._get('/api/settings', { key }); }
  async set(key: string, value: string): Promise<void> { await this.c._post('/api/settings', { key, value }); }
  async getAll(): Promise<Record<string, string>> { return this.c._get('/api/settings/all'); }
}

export class SystemAPI {
  constructor(private readonly c: OverseerClient) {}
  async health(): Promise<ServiceHealth> { return this.c._get('/healthz'); }
  async runtimeHealth(): Promise<RuntimeHealth> { return this.c._get('/api/health'); }
  async load(): Promise<SystemLoad> { return this.c._get('/api/system/load'); }
  async models(): Promise<string[]> { return this.c._getList('/api/models/available'); }
  async registryStatus(): Promise<RuntimeRegistryStatus> { return this.c._get('/api/registry/status'); }
  async refreshRegistry(): Promise<void> { await this.c._post('/api/registry/refresh', {}); }
  async exportFile(filename: string, content: string): Promise<string> { return this.c._post('/api/system/export', { filename, content }); }
}
