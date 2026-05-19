import { useState, useEffect, useCallback } from 'react'
import { Key, Plus, Trash2, Copy, RefreshCw, CheckCircle, Shield, Zap } from 'lucide-react'
import { api, KeyInfo, truncate } from '../api/shield'

export default function KeyManager() {
  const [keys, setKeys] = useState<KeyInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [generating, setGenerating] = useState(false)
  const [label, setLabel] = useState('')
  const [kahf, setKahf] = useState(true)
  const [fortress, setFortress] = useState(true)
  const [kind, setKind] = useState<'bcs521' | 'hybrid'>('bcs521')
  const [copied, setCopied] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      setLoading(true)
      const data = await api.listKeys()
      setKeys(data)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { load() }, [load])

  const generate = async () => {
    setGenerating(true)
    setError(null)
    try {
      await api.generateKey(
        label || `key-${Date.now()}`,
        kahf,
        fortress,
        kind === 'hybrid' ? 'hybrid-bcs521-mlkem1024' : 'bcs521',
      )
      setLabel('')
      await load()
    } catch (e) {
      setError(String(e))
    } finally {
      setGenerating(false)
    }
  }

  const revoke = async (id: string) => {
    if (!confirm('Revoke this key? This cannot be undone.')) return
    try {
      await api.revokeKey(id)
      await load()
    } catch (e) {
      setError(String(e))
    }
  }

  const copy = (text: string, id: string) => {
    navigator.clipboard.writeText(text)
    setCopied(id)
    setTimeout(() => setCopied(null), 2000)
  }

  return (
    <div className="space-y-8 animate-slide-up">
      <div>
        <h1 className="section-title text-2xl">Key Manager</h1>
        <p className="section-sub">Generate and manage BCS-521 / Hybrid keypairs</p>
      </div>

      {/* Generate form */}
      <div className="card">
        <h2 className="font-semibold text-slate-100 mb-4 flex items-center gap-2">
          <Plus className="w-4 h-4 text-emerald-400" /> Generate New Key
        </h2>
        <div className="grid sm:grid-cols-2 gap-4 mb-4">
          <div>
            <label className="label">Label (optional)</label>
            <input
              className="input"
              placeholder="e.g. Halal-Cert-Signer"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
            />
          </div>
          <div>
            <label className="label">Key Type</label>
            <select
              className="input"
              value={kind}
              onChange={(e) => setKind(e.target.value as 'bcs521' | 'hybrid')}
            >
              <option value="bcs521">BCS-521 (Classical)</option>
              <option value="hybrid">Hybrid BCS-521 + ML-KEM-1024 (Post-Quantum)</option>
            </select>
          </div>
        </div>
        <div className="flex flex-wrap gap-6 mb-5">
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={kahf}
              onChange={(e) => setKahf(e.target.checked)}
              className="w-4 h-4 rounded accent-amber-500"
            />
            <span className="text-sm text-slate-300">Kahf-tagged <span className="text-amber-400">(Surah 18 domain separator)</span></span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={fortress}
              onChange={(e) => setFortress(e.target.checked)}
              className="w-4 h-4 rounded accent-emerald-500"
            />
            <span className="text-sm text-slate-300">Fortress-tagged <span className="text-emerald-400">(CT + Zeroize)</span></span>
          </label>
        </div>
        {error && (
          <div className="bg-red-900/30 border border-red-700 rounded-xl p-3 text-red-300 text-sm mb-4">
            {error}
          </div>
        )}
        <button className="btn-primary" onClick={generate} disabled={generating}>
          {generating ? (
            <><RefreshCw className="w-4 h-4 animate-spin" /> Generating…</>
          ) : (
            <><Key className="w-4 h-4" /> Generate Keypair</>
          )}
        </button>
      </div>

      {/* Key list */}
      <div>
        <div className="flex items-center justify-between mb-4">
          <h2 className="font-semibold text-slate-100">
            Keys ({keys.filter((k) => k.active).length} active)
          </h2>
          <button className="btn-secondary text-sm py-1.5" onClick={load}>
            <RefreshCw className="w-3.5 h-3.5" /> Refresh
          </button>
        </div>

        {loading ? (
          <div className="text-center py-12 text-slate-500">Loading…</div>
        ) : keys.length === 0 ? (
          <div className="card text-center py-12">
            <Key className="w-10 h-10 text-slate-600 mx-auto mb-3" />
            <p className="text-slate-400">No keys yet — generate your first keypair above.</p>
          </div>
        ) : (
          <div className="space-y-3">
            {keys.map((k) => (
              <div
                key={k.id}
                className={`card transition-colors ${!k.active ? 'opacity-50 border-slate-700/30' : 'hover:border-emerald-800/60'}`}
              >
                <div className="flex items-start justify-between gap-4 flex-wrap">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1 flex-wrap">
                      <span className="font-semibold text-slate-100 truncate">
                        {k.label || k.id.slice(0, 16) + '…'}
                      </span>
                      {k.active ? (
                        <span className="badge-green">Active</span>
                      ) : (
                        <span className="badge-red">Revoked</span>
                      )}
                      {k.kahf && <span className="badge-gold">Kahf ☾</span>}
                      {k.fortress && <span className="badge-green">Fortress 🛡</span>}
                      {k.kind === 'hybrid-bcs521-mlkem1024' && (
                        <span className="bg-purple-900/50 text-purple-300 border border-purple-700/50 px-2.5 py-0.5 rounded-full text-xs font-medium">PQ Hybrid</span>
                      )}
                    </div>
                    <p className="text-xs text-slate-500 font-mono mb-2">ID: {k.id}</p>
                    <p className="text-xs text-slate-500 mb-2">
                      Created: {new Date(k.created_at).toLocaleString()}
                    </p>
                    <div className="flex items-center gap-2">
                      <p className="text-xs font-mono text-emerald-300 truncate flex-1">
                        PK: {truncate(k.public_key_hex, 20)}
                      </p>
                      <button
                        className="shrink-0 text-slate-400 hover:text-emerald-400 transition-colors"
                        onClick={() => copy(k.public_key_hex, k.id)}
                        title="Copy public key"
                      >
                        {copied === k.id ? (
                          <CheckCircle className="w-4 h-4 text-emerald-400" />
                        ) : (
                          <Copy className="w-4 h-4" />
                        )}
                      </button>
                    </div>
                  </div>
                  {k.active && (
                    <button
                      className="btn-secondary text-sm py-1.5 px-3 text-red-400 hover:text-red-300 hover:bg-red-900/30 shrink-0"
                      onClick={() => revoke(k.id)}
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Info */}
      <div className="grid sm:grid-cols-2 gap-4">
        <div className="card border-amber-800/40">
          <div className="flex items-center gap-2 mb-2">
            <Shield className="w-4 h-4 text-amber-400" />
            <span className="font-medium text-slate-200">BCS-521 Classical</span>
          </div>
          <p className="text-slate-400 text-sm">66-byte secret key · 133-byte public key · ECDLP ≈ 2²⁶⁰ · RFC 6979 ECDSA</p>
        </div>
        <div className="card border-purple-800/40">
          <div className="flex items-center gap-2 mb-2">
            <Zap className="w-4 h-4 text-purple-400" />
            <span className="font-medium text-slate-200">Hybrid BCS-521 + ML-KEM-1024</span>
          </div>
          <p className="text-slate-400 text-sm">Quantum-resistant KEM · NIST FIPS-203 · Kahf-bound transcript · Belt + suspenders</p>
        </div>
      </div>
    </div>
  )
}
