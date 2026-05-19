import { Routes, Route, NavLink, useLocation } from 'react-router-dom'
import { Shield, Key, FilePlus, FileCheck, BookOpen, Activity } from 'lucide-react'
import Home from './pages/Home'
import KeyManager from './pages/KeyManager'
import IssueCert from './pages/IssueCert'
import VerifyCert from './pages/VerifyCert'
import AuditTrail from './pages/AuditTrail'

const NAV = [
  { to: '/', label: 'Home', icon: Shield, exact: true },
  { to: '/keys', label: 'Keys', icon: Key },
  { to: '/issue', label: 'Issue Cert', icon: FilePlus },
  { to: '/verify', label: 'Verify', icon: FileCheck },
  { to: '/audit', label: 'Audit', icon: BookOpen },
]

export default function App() {
  const location = useLocation()

  return (
    <div className="min-h-screen flex flex-col">
      {/* Top bar */}
      <header className="border-b border-slate-800 bg-slate-950/90 backdrop-blur sticky top-0 z-50">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 h-16 flex items-center justify-between gap-4">
          {/* Logo */}
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 bg-emerald-700 rounded-xl flex items-center justify-center text-lg">
              🕌
            </div>
            <div>
              <div className="font-bold text-slate-100 leading-tight text-sm sm:text-base">
                BCS-521 Fortress
              </div>
              <div className="text-[10px] text-amber-400 font-medium leading-tight">
                Islamic Fintech Crypto
              </div>
            </div>
          </div>

          {/* Bismillah */}
          <div className="hidden md:block arabic-text text-amber-400 text-lg gold-glow select-none">
            بِسْمِ اللَّهِ الرَّحْمَنِ الرَّحِيمِ
          </div>

          {/* Live indicator */}
          <div className="flex items-center gap-2">
            <Activity className="w-3.5 h-3.5 text-emerald-400 animate-pulse" />
            <span className="text-xs text-emerald-400 font-medium">Fortress Active</span>
          </div>
        </div>
      </header>

      {/* Nav */}
      <nav className="border-b border-slate-800 bg-slate-900/60">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 flex gap-1 overflow-x-auto py-1">
          {NAV.map(({ to, label, icon: Icon, exact }) => (
            <NavLink
              key={to}
              to={to}
              end={exact}
              className={({ isActive }) =>
                `flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-all duration-150 ${
                  isActive
                    ? 'bg-emerald-800/60 text-emerald-300 border border-emerald-700/50'
                    : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
                }`
              }
            >
              <Icon className="w-4 h-4" />
              {label}
            </NavLink>
          ))}
        </div>
      </nav>

      {/* Content */}
      <main className="flex-1 max-w-7xl mx-auto w-full px-4 sm:px-6 py-8 animate-fade-in">
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/keys" element={<KeyManager />} />
          <Route path="/issue" element={<IssueCert />} />
          <Route path="/verify" element={<VerifyCert />} />
          <Route path="/audit" element={<AuditTrail />} />
        </Routes>
      </main>

      {/* Footer */}
      <footer className="border-t border-slate-800 py-4 text-center text-xs text-slate-600">
        BCS-521 Fortress v0.3.0 — ECDLP ≈ 2²⁶⁰ — PQ Hybrid (ML-KEM-1024) — RFC 6979 ECDSA —{' '}
        <span className="text-amber-700">Bismillah</span>
      </footer>
    </div>
  )
}
