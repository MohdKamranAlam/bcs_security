import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Shield, Zap, Lock, Globe, ChevronRight, CheckCircle, AlertTriangle } from 'lucide-react'
import { api } from '../api/shield'

interface ShieldInfo {
  name: string
  version: string
  curve: string
  security_bits: number
  shariah_compliant: boolean
}

const FEATURES = [
  { icon: Shield, title: 'BCS-521 Curve', desc: 'y² = x³ − 2x² + 5x + 4 — Bismillah master equation. ECDLP ≈ 2²⁶⁰.' },
  { icon: Lock, title: 'Constant-Time', desc: 'Montgomery ladder + Renes-Costello-Batina formulas. No timing leaks.' },
  { icon: Zap, title: 'Post-Quantum Hybrid', desc: 'BCS-521 ECDH + ML-KEM-1024 (NIST FIPS-203). Quantum-resistant KEM.' },
  { icon: Globe, title: 'RFC 6979 ECDSA', desc: 'Deterministic sign/verify. SHA-256 + Kahf domain separator.' },
]

const KAHF_PRIMES = [
  { n: '2141', label: 'First Ayah Position (prime)' },
  { n: '2969', label: 'Last Ayah ZF prime' },
  { n: '373', label: 'Years in Cave ZF prime' },
  { n: '19', label: 'Surah ZF = Bismillah letters' },
  { n: '7', label: 'Sleepers of Cave (prime)' },
]

export default function Home() {
  const nav = useNavigate()
  const [info, setInfo] = useState<ShieldInfo | null>(null)
  const [online, setOnline] = useState<boolean | null>(null)

  useEffect(() => {
    api.health()
      .then(() => api.info().then((d) => { setInfo(d); setOnline(true) }))
      .catch(() => setOnline(false))
  }, [])

  return (
    <div className="space-y-10 animate-slide-up">
      {/* Hero */}
      <section className="islamic-pattern gradient-border rounded-3xl p-8 sm:p-12 text-center relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-br from-emerald-950/80 to-slate-950/90 rounded-3xl" />
        <div className="relative z-10">
          <p className="arabic-text text-amber-400 text-3xl sm:text-4xl gold-glow mb-4">
            بِسْمِ اللَّهِ الرَّحْمَنِ الرَّحِيمِ
          </p>
          <h1 className="text-3xl sm:text-5xl font-bold text-white mb-3 tracking-tight">
            BCS-521 Fortress
          </h1>
          <p className="text-emerald-400 text-lg sm:text-xl font-medium mb-2">
            World's First Qur'an-Transparent Cryptosystem
          </p>
          <p className="text-slate-400 max-w-2xl mx-auto text-sm sm:text-base mb-8">
            Every cryptographic constant is derived from the Bismillah master equation{' '}
            <code className="text-amber-400 bg-slate-800 px-1.5 py-0.5 rounded">T_A = 17B² + 5B + 4 = 6236</code>.
            Post-quantum hybrid KEM. Shariah-compliant audit trail.
          </p>

          {/* Status pill */}
          <div className="flex justify-center mb-8">
            {online === null && (
              <span className="badge-gold px-4 py-1.5 text-sm animate-pulse">Connecting to BCS Shield…</span>
            )}
            {online === true && (
              <div className="flex items-center gap-2 bg-emerald-900/50 border border-emerald-600/50 rounded-full px-5 py-2">
                <span className="w-2 h-2 bg-emerald-400 rounded-full animate-pulse" />
                <span className="text-emerald-300 text-sm font-medium">
                  BCS Shield {info?.version} — Online
                </span>
                <CheckCircle className="w-4 h-4 text-emerald-400" />
              </div>
            )}
            {online === false && (
              <div className="flex items-center gap-2 bg-red-900/50 border border-red-600/50 rounded-full px-5 py-2">
                <AlertTriangle className="w-4 h-4 text-red-400" />
                <span className="text-red-300 text-sm font-medium">
                  BCS Shield offline — start with:{' '}
                  <code className="font-mono">cargo run --manifest-path bcs-shield/Cargo.toml --release</code>
                </span>
              </div>
            )}
          </div>

          <div className="flex flex-wrap justify-center gap-3">
            <button className="btn-primary text-base px-6 py-3" onClick={() => nav('/issue')}>
              Issue Certificate <ChevronRight className="w-4 h-4" />
            </button>
            <button className="btn-secondary text-base px-6 py-3" onClick={() => nav('/verify')}>
              Verify Certificate
            </button>
          </div>
        </div>
      </section>

      {/* Math Identity */}
      <section className="card">
        <h2 className="section-title">The Bismillah Identity</h2>
        <p className="section-sub">The mathematical lock that ties BCS-521 to Surah Al-Kahf</p>
        <div className="grid sm:grid-cols-2 gap-4">
          <div className="bg-slate-800/60 rounded-xl p-5 border border-slate-700">
            <p className="text-slate-400 text-xs mb-2 font-medium uppercase tracking-wider">Master Equation</p>
            <p className="text-amber-300 font-mono text-lg">T_A = 17B² + 5B + 4 = 6236</p>
            <p className="text-slate-400 text-xs mt-2">B = 19 (Bismillah letters), T_A = total ayahs</p>
          </div>
          <div className="bg-slate-800/60 rounded-xl p-5 border border-slate-700">
            <p className="text-slate-400 text-xs mb-2 font-medium uppercase tracking-wider">Bismillah Identity</p>
            <p className="text-emerald-300 font-mono text-lg">#E(F₁₇) = 19</p>
            <p className="text-slate-400 text-xs mt-2">The BCS curve over F₁₇ has exactly 19 points</p>
          </div>
          <div className="bg-slate-800/60 rounded-xl p-5 border border-slate-700">
            <p className="text-slate-400 text-xs mb-2 font-medium uppercase tracking-wider">Curve Equation</p>
            <p className="text-blue-300 font-mono text-lg">y² = x³ − 2x² + 5x + 4</p>
            <p className="text-slate-400 text-xs mt-2">Generator G = (0, 2), cofactor h = 1</p>
          </div>
          <div className="bg-slate-800/60 rounded-xl p-5 border border-slate-700">
            <p className="text-slate-400 text-xs mb-2 font-medium uppercase tracking-wider">Security</p>
            <p className="text-purple-300 font-mono text-lg">ECDLP ≈ 2²⁶⁰</p>
            <p className="text-slate-400 text-xs mt-2">521-bit prime p, group order n prime, h = 1</p>
          </div>
        </div>
      </section>

      {/* Surah Kahf Prime Lock */}
      <section className="card">
        <h2 className="section-title">Surah Al-Kahf Prime Lock</h2>
        <p className="section-sub">5 cryptographic primes derived from Surah 18 — used as Kahf domain separator</p>
        <div className="flex flex-wrap gap-3">
          {KAHF_PRIMES.map(({ n, label }) => (
            <div key={n} className="bg-slate-800 border border-amber-800/40 rounded-xl px-4 py-3 flex items-center gap-3">
              <span className="text-amber-400 font-mono text-xl font-bold">{n}</span>
              <span className="text-slate-400 text-xs max-w-32">{label}</span>
            </div>
          ))}
        </div>
      </section>

      {/* Feature grid */}
      <section>
        <h2 className="text-2xl font-bold text-slate-100 mb-6">Fortress Features</h2>
        <div className="grid sm:grid-cols-2 gap-4">
          {FEATURES.map(({ icon: Icon, title, desc }) => (
            <div key={title} className="card hover:border-emerald-700/60 transition-colors">
              <div className="flex items-start gap-4">
                <div className="w-10 h-10 bg-emerald-900/60 rounded-xl flex items-center justify-center shrink-0">
                  <Icon className="w-5 h-5 text-emerald-400" />
                </div>
                <div>
                  <h3 className="font-semibold text-slate-100 mb-1">{title}</h3>
                  <p className="text-slate-400 text-sm leading-relaxed">{desc}</p>
                </div>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* Quick start */}
      <section className="card border-amber-800/40">
        <h2 className="section-title">Quick Start</h2>
        <p className="section-sub">Issue your first BCS-521 signed Halal Certificate in 3 steps</p>
        <div className="space-y-3">
          {[
            { step: '1', action: 'Generate Key', detail: 'Create a BCS-521 keypair in Key Manager', page: '/keys' },
            { step: '2', action: 'Issue Certificate', detail: 'Fill in product details and sign with BCS-521', page: '/issue' },
            { step: '3', action: 'Verify Certificate', detail: 'Verify signature with public key', page: '/verify' },
          ].map(({ step, action, detail, page }) => (
            <button
              key={step}
              onClick={() => nav(page)}
              className="w-full flex items-center gap-4 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-xl p-4 text-left transition-colors group"
            >
              <span className="w-9 h-9 bg-emerald-700 rounded-full flex items-center justify-center text-white font-bold shrink-0">
                {step}
              </span>
              <div className="flex-1">
                <p className="font-semibold text-slate-100">{action}</p>
                <p className="text-slate-400 text-sm">{detail}</p>
              </div>
              <ChevronRight className="w-4 h-4 text-slate-500 group-hover:text-emerald-400 transition-colors" />
            </button>
          ))}
        </div>
      </section>
    </div>
  )
}
