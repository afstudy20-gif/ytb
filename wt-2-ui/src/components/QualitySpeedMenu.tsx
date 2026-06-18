interface MenuItem {
  label: string
  value: string
}

interface QualitySpeedMenuProps {
  label: string
  items: MenuItem[]
  selected: string
  onSelect: (value: string) => void
  onClose: () => void
}

export function QualitySpeedMenu({ label, items, selected, onSelect, onClose }: QualitySpeedMenuProps) {
  return (
    <div className="absolute bottom-12 right-0 z-50 min-w-[10rem] rounded-lg bg-black/90 p-2 text-white shadow-lg">
      <div className="mb-1 px-2 text-xs font-semibold text-subtext">{label}</div>
      {items.map((item) => (
        <button
          key={item.value}
          type="button"
          onClick={() => {
            onSelect(item.value)
            onClose()
          }}
          className={`block w-full rounded px-2 py-1.5 text-left text-sm ${
            selected === item.value ? 'bg-accent text-white' : 'hover:bg-white/10'
          }`}
        >
          {item.label}
        </button>
      ))}
    </div>
  )
}
