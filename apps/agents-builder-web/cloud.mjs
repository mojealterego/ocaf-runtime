/**
 * OCAF 0.52 — same-origin client for the authenticated Rust Builder HTTP boundary.
 * This module does NOT authenticate the user or execute agents. The TLS gateway
 * must bind OAuth access tokens to AuthenticatedSubject on the server.
 */
import {decodeDsl} from './editor.mjs';
const IDENTIFIER = /^[A-Za-z0-9_.-]{1,64}$/;
const SHA256 = /^[0-9a-f]{64}$/;
const MAX_RESPONSE = 128 * 1024;
const MAX_REQUEST = 128 * 1024;

export class CloudApiError extends Error {
  constructor(status) {
    super('Cloud API HTTP ' + status + (status === 409 ? ' — konflikt rewizji; odśwież projekt bez nadpisywania' : ''));
    this.name = 'CloudApiError';
    this.status = status;
  }
}

const identifier = value => {
  if (typeof value !== 'string' || !IDENTIFIER.test(value) || value === '.' || value === '..') {
    throw new Error('identifier: invalid cloud tenant or project');
  }
  return value;
};
const verifyHead = head => {
  if (!head || !Number.isSafeInteger(head.revision) || head.revision < 1 ||
      typeof head.sha256 !== 'string' || !SHA256.test(head.sha256)) {
    throw new Error('head: invalid cloud revision receipt');
  }
  return head;
};
const verifyProject = (project, tenant, projectId) => {
  if (!project || typeof project !== 'object' || project.tenant_id !== tenant ||
      project.project_id !== projectId || !Number.isSafeInteger(project.revision)) {
    throw new Error('tenant: invalid cloud project identity');
  }
  return decodeDsl(JSON.stringify(project), tenant);
};

export class CloudWorkspaceClient {
  constructor({token, origin, fetchImpl = globalThis.fetch.bind(globalThis)}) {
    if (typeof token !== 'string' || !token || token.length > 4096 || /[^\x21-\x7e]/.test(token)) {
      throw new Error('oauth: invalid access token');
    }
    const url = new URL(origin);
    if (url.origin !== origin || url.username || url.password || url.pathname !== '/' ||
        (url.protocol !== 'https:' && !(url.protocol === 'http:' && ['localhost','127.0.0.1'].includes(url.hostname)))) {
      throw new Error('origin: Cloud API requires same-origin HTTPS (or loopback for local tests)');
    }
    this.origin = origin;
    this.token = token; // ephemeral only; never persist in localStorage, URL or logs
    this.fetchImpl = fetchImpl;
  }

  async request(method, path, payload) {
    const body = payload === undefined ? undefined : JSON.stringify(payload);
    if (body !== undefined && new TextEncoder().encode(body).length > MAX_REQUEST) {
      throw new Error('limit: Cloud API request exceeds 128 KiB');
    }
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 10000);
    try {
      const response = await this.fetchImpl(this.origin + path, {
        method, credentials:'omit', redirect:'error', cache:'no-store',
        referrerPolicy:'no-referrer', signal:controller.signal,
        headers: {
          Authorization:'Bearer ' + this.token,
          ...(body === undefined ? {} : {'Content-Type':'application/json'})
        },
        ...(body === undefined ? {} : {body})
      });
      if (!response.ok) throw new CloudApiError(response.status);
      if (Number(response.headers.get('Content-Length')) > MAX_RESPONSE) throw new Error('limit: oversized cloud response');
      const source = await response.text();
      if (new TextEncoder().encode(source).length > MAX_RESPONSE) throw new Error('limit: oversized cloud response');
      return JSON.parse(source);
    } finally {
      clearTimeout(timer);
    }
  }

  async load(tenant, projectId) {
    identifier(tenant); identifier(projectId);
    const response = await this.request('GET', '/v1/projects/' + encodeURIComponent(projectId));
    const project = verifyProject(response.project, tenant, projectId);
    verifyHead(response.head);
    if (project.revision !== response.head.revision) throw new Error('revision: cloud head mismatch');
    return {project, head:response.head};
  }

  async save(tenant, project, expected) {
    identifier(tenant); verifyProject(project, tenant, project.project_id);
    if (expected !== null) verifyHead(expected);
    const requiredRevision = expected === null ? 1 : expected.revision + 1;
    if (project.revision !== requiredRevision) throw new Error('revision: cloud CAS requires one new revision');
    const head = await this.request('PUT', '/v1/projects/' + encodeURIComponent(identifier(project.project_id)), {project, expected});
    verifyHead(head);
    if (head.revision !== project.revision) throw new Error('revision: cloud write receipt mismatch');
    return head;
  }

  async preview(tenant, project) {
    identifier(tenant); verifyProject(project, tenant, project.project_id);
    const result = await this.request('POST', '/v1/projects/preview', project);
    if (!result || !Array.isArray(result.ordered_nodes) || !Array.isArray(result.requested_tools) ||
        !SHA256.test(result.project_sha256) || !Number.isSafeInteger(result.max_cost_minor)) {
      throw new Error('schema: invalid cloud preview receipt');
    }
    return result;
  }
}
