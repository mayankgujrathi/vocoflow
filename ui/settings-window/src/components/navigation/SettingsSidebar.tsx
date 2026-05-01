import type { IconType } from 'react-icons'

export type SettingsTab = 'general' | 'logging' | 'transcription' | 'history' | 'about'

type SidebarItem = {
  id: SettingsTab
  label: string
  icon: IconType
}

type SettingsSidebarProps = {
  items: SidebarItem[]
  activeTab: SettingsTab
  onSelect: (tab: SettingsTab) => void
}

export function SettingsSidebar({ items, activeTab, onSelect }: SettingsSidebarProps) {
  return (
    <aside className="rounded-2xl border border-slate-700/60 bg-glass p-2 backdrop-blur">
      <nav className="flex flex-col gap-2">
        {items.map((item) => {
          const selected = item.id === activeTab
          const Icon = item.icon
          return (
            <button
              key={item.id}
              type="button"
              onClick={() => onSelect(item.id)}
              className={`group flex flex-col items-center gap-1 rounded-xl border p-2 text-[11px] transition ${
                selected
                  ? 'border-cyan-300/40 bg-material-gradient text-white shadow-glow'
                  : 'border-slate-700/60 bg-slate-900/60 text-slate-300 hover:border-slate-500'
              }`}
            >
              <Icon className="text-lg" />
              <span>{item.label}</span>
            </button>
          )
        })}
      </nav>
    </aside>
  )
}
