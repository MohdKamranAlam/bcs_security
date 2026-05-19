import { useState } from 'react'
import { FileCheck, CheckCircle, XCircle, AlertCircle } from 'lucide-react'
import { api, toHex, truncate } from '../api/shield'

export default function VerifyCert() {
  const [pkHex, setPkHex] = useState('')
  const [message, setMessage] = useState('')
  const [sigHex, setSigHex] = useState('')
  const [verifying, setVerifying] = useState(false)
  const [result, setResult] = useState<{ valid: boolean; proof: string } | null>(null)
  const [error, setError] = useState<string | null>(null)

  const verify = async () => {
    if (!pkHex || !message || !sigHex) return
    setVerifying(true)
    setError(null)
    setResult(null)
    try {
      const msgHex = toHex(message)
      const res = await api.verify(pkHex, msgHex, sigHex)
      setResult({ valid: res.valid, proof: res.proof_id })
    } catch (e) {
      setError(String(e))
    } finally {
      setVerifying(false)
    }
  }

  return (
    <div className="space-y-8 animate-slide-up">
      <div>
        <h1 className="section-title text-2xl">Verify Certificate</h1>
        <p className="section-sub">Verify a BCS-521 signature against a public key and message</p>
      </div>

      <div className="grid lg:grid-cols-5 gap-6">
        <div className="lg:col-span-3 card">
          <h2 className="font-semibold text-slate-100 mb-5 flex items-center gap-2">
            <FileCheck className="w-4 h-4 text-emerald-400" /> Verification Input
          </h2>

          <div className="space-y-4">
            <div>
              <label className="label">Public Key (hex)</label>
              <textarea
                className="textarea"
                rows={3}
                placeholder="04...133 hex chars for BCS-521 uncompressed point"
                value={pkHex}
                onChange={(e) => setPkHex(e.target.value.trim())}
              />
              <p className="text-xs text-slate-500 mt-1">{pkHex.length} chars</p>
            </div>

            <div>
              <label className="label">Original Message (plaintext)</label>
              <textarea
                className="textarea"
                rows={3}
                placeholder="The original certificate content that was signed"
                value={message}
                onChange={(e) => setMessage(e.target.value)}
              />
            </div>

            <div>
              <label className="label">Signature (hex)</label>
              <textarea
                className="textarea"
                rows={3}
                placeholder="132 hex chars (66 bytes) for BCS-521 ECDSA signature"
                value={sigHex}
                onChange={(e) => setSigHex(e.target.value.trim())}
              />
              <p className="text-xs text-slate-500 mt-1">{sigHex.length} chars</p>
            </div>
          </div>

          {error && (
            <div className="bg-red-900/30 border border-red-700 rounded-xl p-3 text-red-300 text-sm mt-4">
              <AlertCircle className="w-4 h-4 inline mr-1" />{error}
            </div>
          )}

          <button
            className="btn-primary mt-5"
            onClick={verify}
            disabled={verifying || !pkHex || !message || !sigHex}
          >
            {verifying ? 'Verifying…' : <><FileCheck className="w-4 h-4" /> Verify Signature</>}
          </button>
        </div>

        {/* Result */}
        <div className="lg:col-span-2 space-y-4">
          {result && (
            <div className={`card animate-slide-up ${result.valid ? 'border-emerald-700/60' : 'border-red-700/60'}`}>
              {result.valid ? (
                <div className="text-center py-6">
                  <div className="w-16 h-16 bg-emerald-900/60 rounded-full flex items-center justify-center mx-auto mb-4">
                    <CheckCircle className="w-8 h-8 text-emerald-400" />
                  </div>
                  <h3 className="text-xl font-bold text-emerald-400 mb-2">VALID</h3>
                  <p className="text-slate-400 text-sm mb-4">The signature is authentic. Certificate verified with BCS-521.</p>
                  <div className="bg-slate-800/80 rounded-xl p-3 text-left">
                    <p className="text-xs text-slate-500 mb-1">Execution Proof ID</p>
                    <p className="text-xs font-mono text-purple-300">{result.proof}</p>
                  </div>
                </div>
              ) : (
                <div className="text-center py-6">
                  <div className="w-16 h-16 bg-red-900/60 rounded-full flex items-center justify-center mx-auto mb-4">
                    <XCircle className="w-8 h-8 text-red-400" />
                  </div>
                  <h3 className="text-xl font-bold text-red-400 mb-2">INVALID</h3>
                  <p className="text-slate-400 text-sm mb-4">The signature does not match. This certificate may be tampered or forged.</p>
                  <div className="bg-slate-800/80 rounded-xl p-3 text-left">
                    <p className="text-xs text-slate-500 mb-1">Execution Proof ID</p>
                    <p className="text-xs font-mono text-purple-300">{result.proof}</p>
                  </div>
                </div>
              )}
            </div>
          )}

          {!result && (
            <div className="card text-center py-12">
              <FileCheck className="w-10 h-10 text-slate-600 mx-auto mb-3" />
              <p className="text-slate-400 text-sm">Enter public key, message, and signature to verify.</p>
            </div>
          )}

          {/* Help */}
          <div className="card border-slate-700/40">
            <h3 className="font-semibold text-slate-300 text-sm mb-3">How Verification Works</h3>
            <ol className="space-y-2 text-xs text-slate-400">
              <li className="flex gap-2">
                <span className="text-emerald-400 font-bold shrink-0">1.</span>
                <span>Message is converted to hex and hashed with SHA-256</span>
              </li>
              <li className="flex gap-2">
                <span className="text-emerald-400 font-bold shrink-0">2.</span>
                <span>BCS-521 ECDSA verify checks r, s against public key Q</span>
              </li>
              <li className="flex gap-2">
                <span className="text-emerald-400 font-bold shrink-0">3.</span>
                <span>Fortress execution proof recorded in audit log</span>
              </li>
              <li className="flex gap-2">
                <span className="text-emerald-400 font-bold shrink-0">4.</span>
                <span>Kahf domain separator ensures protocol binding</span>
              </li>
            </ol>
          </div>
        </div>
      </div>
    </div>
  )
}
