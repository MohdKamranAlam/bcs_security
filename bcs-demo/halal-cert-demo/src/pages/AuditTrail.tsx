import { useState, useEffect, useCallback } from 'react'
import { BookOpen, RefreshCw, Shield, CheckCircle, XCircle, Clock } from 'lucide-react'
import { api, AuditEntry, ComplianceReport } from '../api/shield'

export default function AuditTrail() {
  const [entries, setEntries] = useState<AuditEntry[]>([])
  const [report, setReport] = useState<ComplianceReport | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [tab, setTab] = useState<'log' | 'compliance'>('log')

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const [logData, reportData] = await Promise.all([api.auditLog(), api.compliance()])
      setEntries(logData)
      setReport(reportData)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { load() }, [load])

  return (
    <div className="space-y-8 animate-slide-up">
      <div className="flex items-center justify-between flex-wrap gap-4">
        <div>
          <h1 className="section-title text-2xl">Shariah Audit Trail</h1>
          <p className="section-sub">Fortress execution proofs + Shariah compliance report</p>
        </div>
        <button className="btn-secondary text-sm" onClick={load}>
          <RefreshCw className="w-3.5 h-3.5" /> Refresh
        </button>
      </div>

      {/* Tabs */}
      <div className="flex gap-1 bg-slate-900 rounded-xl p-1 w-fit">
        {(['log', 'compliance'] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-all ${
              tab === t
                ? 'bg-emerald-800/60 text-emerald-300 border border-emerald-700/50'
                : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            {t === 'log' ? 'Audit Log' : 'Compliance Report'}
          </button>
        ))}
      </div>

      {error && (
        <div className="bg-red-900/30 border border-red-700 rounded-xl p-3 text-red-300 text-sm">
          {error}
        </div>
      )}

      {loading ? (
        <div className="text-center py-12 text-slate-500">Loading audit data…</div>
      ) : tab === 'log' ? (
        /* Audit Log */
        <div className="space-y-3">
          {entries.length === 0 ? (
            <div className="card text-center py-12">
              <BookOpen className="w-10 h-10 text-slate-600 mx-auto mb-3" />
              <p className="text-slate-400">No audit entries yet. Sign or verify a certificate to generate entries.</p>
            </div>
          ) : (
            entries.map((e) => (
              <div key={e.id} className="card hover:border-slate-600 transition-colors">
                <div className="flex items-start justify-between gap-4 flex-wrap">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1 flex-wrap">
                      <span className="font-semibold text-slate-100">{e.operation}</span>
                      {e.success ? (
                        <span className="badge-green"><CheckCircle className="w-3 h-3 inline mr-1" />Success</span>
                      ) : (
                        <span className="badge-red"><XCircle className="w-3 h-3 inline mr-1" />Failed</span>
                      )}
                    </div>
                    {e.key_id && (
                      <p className="text-xs text-slate-500 font-mono">Key: {e.key_id}</p>
                    )}
                    <p className="text-xs text-slate-500 font-mono">Proof: {e.proof_id}</p>
                    {e.fortress_flags && (
                      <p className="text-xs text-emerald-400 mt-1">
                        Fortress: {e.fortress_flags}
                      </p>
                    )}
                  </div>
                  <div className="flex items-center gap-1 text-xs text-slate-500 shrink-0">
                    <Clock className="w-3 h-3" />
                    {new Date(e.timestamp).toLocaleString()}
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      ) : (
        /* Compliance Report */
        report && (
          <div className="space-y-6">
            {/* Summary cards */}
            <div className="grid sm:grid-cols-4 gap-4">
              <div className="card text-center">
                <p className="text-3xl font-bold text-slate-100">{report.total_operations}</p>
                <p className="text-xs text-slate-400">Total Operations</p>
              </div>
              <div className="card text-center border-emerald-800/40">
                <p className="text-3xl font-bold text-emerald-400">{report.fortress_operations}</p>
                <p className="text-xs text-slate-400">Fortress Ops</p>
              </div>
              <div className="card text-center border-amber-800/40">
                <p className="text-3xl font-bold text-amber-400">{report.kahf_operations}</p>
                <p className="text-xs text-slate-400">Kahf-bound</p>
              </div>
              <div className={`card text-center ${report.shariah_compliant ? 'border-emerald-700/60' : 'border-red-700/60'}`}>
                {report.shariah_compliant ? (
                  <>
                    <CheckCircle className="w-8 h-8 text-emerald-400 mx-auto mb-1" />
                    <p className="text-sm font-bold text-emerald-400">COMPLIANT</p>
                  </>
                ) : (
                  <>
                    <XCircle className="w-8 h-8 text-red-400 mx-auto mb-1" />
                    <p className="text-sm font-bold text-red-400">NON-COMPLIANT</p>
                  </>
                )}
                <p className="text-xs text-slate-400">Shariah Status</p>
              </div>
            </div>

            {/* Details */}
            <div className="card">
              <h3 className="font-semibold text-slate-100 mb-4 flex items-center gap-2">
                <Shield className="w-4 h-4 text-emerald-400" /> Compliance Details
              </h3>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-slate-700">
                      <th className="text-left py-2 px-3 text-slate-400 font-medium">Requirement</th>
                      <th className="text-center py-2 px-3 text-slate-400 font-medium">Status</th>
                      <th className="text-left py-2 px-3 text-slate-400 font-medium">Evidence</th>
                    </tr>
                  </thead>
                  <tbody>
                    {report.details.map((d, i) => (
                      <tr key={i} className="border-b border-slate-800">
                        <td className="py-2.5 px-3 text-slate-200">{d.requirement}</td>
                        <td className="py-2.5 px-3 text-center">
                          {d.satisfied ? (
                            <span className="badge-green">✓ Satisfied</span>
                          ) : (
                            <span className="badge-red">✗ Not Met</span>
                          )}
                        </td>
                        <td className="py-2.5 px-3 text-slate-400 text-xs">{d.evidence}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>

            <p className="text-xs text-slate-600 text-center">
              Report generated: {new Date(report.generated_at).toLocaleString()}
            </p>
          </div>
        )
      )}
    </div>
  )
}
