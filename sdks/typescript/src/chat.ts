import { decodeJson, idToDecimal } from './json.js';
import type { ApiId, IdInput } from './types.js';
export interface ApprovalEvent { type: 'approval'; id: string; tool: string; kind: string; summary: string; reason: string }
export type ChatEvent = { type: 'session'; conversation_id: ApiId } | { type: 'token'; text: string } | ApprovalEvent | { type: 'done'; response: string } | { type: 'error'; error: string };
export interface ChatRequest { conversationId?: IdInput; message: string; model?: string }
export interface ChatOptions {
  signal?: AbortSignal;
  /** Idle receive timeout in milliseconds; excludes the approval callback. */
  timeout?: number;
  /** Explicit human decision. Without this callback approvals are rejected. */
  onApproval?: (approval: ApprovalEvent) => boolean | Promise<boolean>;
}
export class ChatAPI {
  constructor(private readonly baseUrl: string, private readonly token: () => string | undefined,
    private readonly factory: (url: string) => WebSocket = url => new WebSocket(url)) {}
  /** One socket per turn. Breaking the iterator closes the socket and cancels the run. */
  async *stream(request: ChatRequest, options: ChatOptions = {}): AsyncGenerator<ChatEvent> {
    const id = idToDecimal(request.conversationId ?? 0);
    if (!request.message.trim() || new TextEncoder().encode(request.message).length > 65536) throw new RangeError('message must contain 1–65536 bytes');
    const timeout = options.timeout ?? 120_000;
    if (!Number.isFinite(timeout) || timeout <= 0) throw new RangeError('timeout must be positive');
    options.signal?.throwIfAborted();
    const url = new URL('/api/ws/chat', this.baseUrl);url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
    const socket = this.factory(url.toString());
    const queue: ChatEvent[] = [];
    let failure: Error | undefined;let wake: (() => void) | undefined;
    const fail = (error: Error) => { failure = error; wake?.(); };
    const onMessage = (event: MessageEvent) => {
      try {
        const frame = decodeJson(String(event.data)) as ChatEvent;
        if (!frame || typeof frame !== 'object' || typeof frame.type !== 'string') throw new Error('Invalid chat event');
        if (queue.length >= 1024) throw new Error('Chat event buffer exceeded');
        queue.push(frame);wake?.();
      } catch (error) { fail(error instanceof Error ? error : new Error(String(error))); }
    };
    const onClose = () => fail(new Error('Chat connection closed before completion'));
    const onError = () => fail(new Error('Chat connection failed'));
    const onAbort = () => { fail(new DOMException('Chat cancelled', 'AbortError'));socket.close(); };
    socket.addEventListener('message', onMessage);socket.addEventListener('close', onClose);socket.addEventListener('error', onError);
    options.signal?.addEventListener('abort', onAbort, { once: true });
    let timer: ReturnType<typeof setTimeout> | undefined;
    const wait = async () => {
      await new Promise<void>(resolve => { wake = resolve;timer = setTimeout(() => fail(new Error('Chat receive timed out')), timeout);if (failure) resolve(); });
      clearTimeout(timer);wake = undefined;
      if (failure && !queue.length) throw failure;
    };
    const onOpen = () => wake?.();socket.addEventListener('open', onOpen);
    try {
      if (socket.readyState !== 1) await wait();
      if (failure) throw failure;
      const token = this.token();if (token) socket.send(JSON.stringify({ token }));
      socket.send(JSON.stringify({ conversation_id: id, message: request.message, model: request.model }));
      while (true) {
        if (failure && !queue.length) throw failure;
        if (!queue.length) await wait();
        const event = queue.shift();if (!event) continue;
        yield event;
        if (event.type === 'error') throw new Error(event.error);
        if (event.type === 'done') return;
        if (event.type === 'approval') {
          let cancel: (() => void) | undefined;
          let approved: boolean;
          try {
            const decision = options.onApproval ? options.onApproval(event) : false;
            const aborted = new Promise<never>((_, reject) => {
              cancel = () => reject(new DOMException('Chat cancelled', 'AbortError'));
              options.signal?.addEventListener('abort', cancel, { once: true });
              if (options.signal?.aborted) cancel();
            });
            approved = await Promise.race([Promise.resolve(decision), aborted]);
          } finally {
            if (cancel) options.signal?.removeEventListener('abort', cancel);
          }
          options.signal?.throwIfAborted();
          socket.send(JSON.stringify({ [approved === true ? 'approve' : 'reject']: event.id }));
        }
      }
    } finally {
      clearTimeout(timer);options.signal?.removeEventListener('abort', onAbort);
      socket.removeEventListener('open', onOpen);socket.removeEventListener('message', onMessage);socket.removeEventListener('close', onClose);socket.removeEventListener('error', onError);socket.close();
    }
  }
}
