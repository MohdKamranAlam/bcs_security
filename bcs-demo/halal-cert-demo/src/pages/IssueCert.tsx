import { useState, useEffect } from 'react'
import { FilePlus, Key, Copy, CheckCircle, AlertCircle } from 'lucide-react'
import { api, KeyInfo, toHex, truncate } from '../api/shield'

const CERT_TYPES = [
  { value: 'halal-product', label: 'Halal Product Certification' },
  { value: 'nikah-nama', label: 'Nikah-Nama (Marriage Contract)' },
  { value: 'quran-translation', label: 'Quran Translation Provenance' },
  { value: 'sukuk', label: 'Sukuk (Islamic Bond)' },
  { value: 'zakat', label: 'Zakat Receipt' },
  { value: 'waqf', label: 'Waqf (Endowment) Deed' },
  { value: 'custom', label: 'Custom Certificate' },
]

export default function IssueCert() {
  const [keys, setKeys] = useState<KeyInfo[]>([])
  const [selectedKey, setSelectedKey] = useState('')
  const [certType, setCertType] = useState('halal-product')
  const [productName, setProductName] = useState('')
  const [issuerName, setIssuerName] = useState('')
  const [additionalData, setAdditionalData] = useState('')
  const [signing, setSigning] = useState(false)
  const [result, setResult] = useState<{ sig: string; proof: string; algo: string } | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    api.listKeys().then((k) => setKeys(k.filter((k) => k.active))).catch(() => {})
  }, [])

  const activeKeys = keys.filter((k) => k.active)

  const issue = async () => {
    if (!selectedKey || !productName || !issuerName) return
    setSigning(true)
    setError(null)
    setResult(null)
    try {
      const payload = JSON.stringify({
        type: certType,
        product: productName,
        issuer: issuerName,
        timestamp: new Date().toISOString(),
        curve: 'BCS-521',
        kahf_bound: true,
        extra: additionalData || undefined,
      })
      const msgHex = toHex(payload)
      const sig = await api.sign(selectedKey, msgHex)
      setResult({ sig: sig.signature_hex, proof: sig.proof_id, algo: sig.algorithm })
    } catch (e) {
      setError(String(e))
    } finally {
      setSigning(false)
    }
  }

  const copySig = () => {
    if (!result) return
    navigator.clipboard.writeText(result.sig)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="space-y-8 animate-slide-up">
      <div>
        <h1 className="section-title text-2xl">Issue Certificate</h1>
        <p className="section-sub">Sign a Halal certificate with BCS-521 + Kahf domain separator</p>
      </div>

      <div className="grid lg:grid-cols-5 gap-6">
        {/* Form */}
        <div className="lg:col-span-3 card">
          <h2 className="font-semibold text-slate-100 mb-5 flex items-center gap-2">
            <FilePlus className="w-4 h-4 text-emerald-400" /> Certificate Details
          </h2>

          <div className="space-y-4">
            <div>
              <label className="label">Certificate Type</label>
              <select className="input" value={certType} onChange={(e) => setCertType(e.target.value)}>
                {CERT_TYPES.map((c) => (
                  <option key={c.value} value={c.value}>{c.label}</option>
                ))}
              </select>
            </div>

            <div>
              <label className="label">Signing Key</label>
              {activeKeys.length === 0 ? (
                <div className="bg-amber-900/30 border border-amber-700 rounded-xl p-3 text-amber-300 text-sm">
                  No active keys — generate one in Key Manager first.
                </div>
              ) : (
                <select className="input" value={selectedKey} onChange={(e) => setSelectedKey(e.target.value)}>
                  <option value="">Select a key…</option>
                  {activeKeys.map((k) => (
                    <option key={k.id} value={k.id}>
                      {k.label || k.id.slice(0, 16)} ({k.kind === 'hybrid-bcs521-mlkem1024' ? 'PQ Hybrid' : 'BCS-521'})
                      {k.kahf ? ' ☾ Kahf' : ''} {k.fortress ? ' 🛡 Fortress' : ''}
                    </option>
                  ))}
                </select>
              )}
            </div>

            <div>
              <label className="label">Product / Certificate Name</label>
              <input
                className="input"
                placeholder="e.g. Tayyabaat Organic Chicken"
                value={productName}
                onChange={(e) => setProductName(e.target.value)}
              />
            </div>

            <div>
              <label className="label">Issuer Name / Organization</label>
              <input
                className="input"
                placeholder="e.g. Jamiat Ulama-i-Hind Halal Trust"
                value={issuerName}
                onChange={(e) => setIssuerName(e.target.value)}
              />
            </div>

            <div>
              <label className="label">Additional Data (optional)</label>
              <textarea
                className="textarea"
                rows={3}
                placeholder="Batch number, expiry date, certifier ID…"
                value={additionalData}
                onChange={(e) => setAdditionalData(e.target.value)}
              />
            </div>
          </div>

          {error && (
            <div className="bg-red-900/30 border border-red-700 rounded-xl p-3 text-red-300 text-sm mt-4">
              <AlertCircle className="w-4 h-4 inline mr-1" />{error}
            </div>
          )}

          <button
            className="btn-gold mt-5"
            onClick={issue}
            disabled={signing || !selectedKey || !productName || !issuerName}
          >
            {signing ? 'Signing with BCS-521…' : (
              <><Key className="w-4 h-4" /> Sign & Issue Certificate</>
            )}
          </button>
        </div>

        {/* Preview / Result */}
        <div className="lg:col-span-2 space-y-4">
          {/* Certificate preview */}
          <div className="card border-amber-800/40">
            <h3 className="font-semibold text-amber-400 mb-3 text-sm uppercase tracking-wider">Certificate Preview</h3>
            <div className="bg-slate-800/80 rounded-xl p-4 space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-slate-500">Type</span>
                <span className="text-slate-200">{CERT_TYPES.find((c) => c.value === certType)?.label}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Product</span>
                <span className="text-slate-200">{productName || '—'}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Issuer</span>
                <span className="text-slate-200">{issuerName || '—'}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Curve</span>
                <span className="text-emerald-400 font-mono">BCS-521</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Kahf-bound</span>
                <span className="text-amber-400">✓</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Timestamp</span>
                <span className="text-slate-200 font-mono text-xs">{new Date().toISOString().slice(0, 19)}</span>
              </div>
            </div>
          </div>

          {/* Signature result */}
          {result && (
            <div className="card border-emerald-800/60 animate-slide-up">
              <h3 className="font-semibold text-emerald-400 mb-3 flex items-center gap-2">
                <CheckCircle className="w-4 h-4" /> Certificate Signed!
              </h3>
              <div className="space-y-3">
                <div>
                  <p className="text-xs text-slate-500 mb-1">Algorithm</p>
                  <p className="text-sm text-slate-200">{result.algo}</p>
                </div>
                <div>
                  <p className="text-xs text-slate-500 mb-1">Signature (hex)</p>
                  <div className="mono-box flex items-start gap-2">
                    <span className="flex-1 break-all">{truncate(result.sig, 40)}</span>
                    <button onClick={copySig} className="shrink-0 text-slate-400 hover:text-emerald-400">
                      {copied ? <CheckCircle className="w-4 h-4 text-emerald-400" /> : <Copy className="w-4 h-4" />}
                    </button>
                  </div>
                </div>
                <div>
                  <p className="text-xs text-slate-500 mb-1">Execution Proof ID</p>
                  <p className="text-xs font-mono text-purple-300">{result.proof}</p>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
