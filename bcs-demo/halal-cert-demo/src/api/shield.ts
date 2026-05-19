const BASE = '/api/v1'

export interface KeyInfo {
  id: string
  kind: string
  public_key_hex: string
  kahf: boolean
  fortress: boolean
  label: string | null
  created_at: string
  active: boolean
}

export interface SignResponse {
  signature_hex: string
  proof_id: string
  algorithm: string
}

export interface VerifyResponse {
  valid: boolean
  proof_id: string
}

export interface AuditEntry {
  id: string
  operation: string
  key_id: string | null
  fortress_flags: string
  proof_id: string
  timestamp: string
  success: boolean
}

export interface ComplianceItem {
  requirement: string
  satisfied: boolean
  evidence: string
}

export interface ComplianceReport {
  generated_at: string
  total_operations: number
  fortress_operations: number
  kahf_operations: number
  shariah_compliant: boolean
  details: ComplianceItem[]
}

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(BASE + path, {
    headers: { 'Content-Type': 'application/json' },
    ...options,
  })
  if (!res.ok) {
    const text = await res.text()
    throw new Error(`API ${res.status}: ${text}`)
  }
  return res.json() as Promise<T>
}

export const api = {
  health: () => request<{ status: string; version: string; fortress_active: boolean }>('/health'),

  info: () => request<{ name: string; version: string; curve: string; security_bits: number; shariah_compliant: boolean }>('/info'),

  generateKey: (label: string, kahf: boolean, fortress: boolean, kind?: string) =>
    request<KeyInfo>('/keys/generate', {
      method: 'POST',
      body: JSON.stringify({ label, kahf, fortress, kind }),
    }),

  listKeys: () => request<KeyInfo[]>('/keys'),

  getKey: (id: string) => request<KeyInfo>(`/keys/${id}`),

  revokeKey: (id: string) =>
    request<{ revoked: boolean }>(`/keys/${id}`, { method: 'DELETE' }),

  sign: (key_id: string, message_hex: string) =>
    request<SignResponse>('/crypto/sign', {
      method: 'POST',
      body: JSON.stringify({ key_id, message_hex }),
    }),

  verify: (public_key_hex: string, message_hex: string, signature_hex: string) =>
    request<VerifyResponse>('/crypto/verify', {
      method: 'POST',
      body: JSON.stringify({ public_key_hex, message_hex, signature_hex }),
    }),

  auditLog: () => request<AuditEntry[]>('/audit/log'),

  compliance: () => request<ComplianceReport>('/audit/compliance'),
}

export function toHex(str: string): string {
  return Array.from(new TextEncoder().encode(str))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('')
}

export function fromHex(hex: string): string {
  const bytes = hex.match(/.{1,2}/g)?.map((b) => parseInt(b, 16)) ?? []
  return new TextDecoder().decode(new Uint8Array(bytes))
}

export function truncate(hex: string, chars = 24): string {
  if (hex.length <= chars * 2) return hex
  return hex.slice(0, chars) + '…' + hex.slice(-8)
}
