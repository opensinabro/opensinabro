import { useEffect, useState } from 'react'
import { Settings2, X } from 'lucide-react'

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { EXAMPLES } from '@/examples'
import { THEMES } from '@/lib/themes'
import { usePlaygroundStore } from '@/store'

interface SettingField {
  id: string
  label: string
  value: string
  onValueChange: (value: string) => void
  disabled?: boolean
  options: { id: string; label: string }[]
  /** 데스크톱 인라인 배치에서 이 셀렉트가 차지할 너비. */
  width: string
}

function SettingSelect({ field, className }: { field: SettingField; className: string }) {
  return (
    <Select value={field.value} onValueChange={field.onValueChange} disabled={field.disabled}>
      <SelectTrigger className={className}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {field.options.map((option) => (
          <SelectItem key={option.id} value={option.id}>
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

/**
 * 모바일 설정 시트. 좁은 화면에서 헤더는 제목만 남기고, 예제·백엔드·테마는
 * 아래에서 올라오는 이 시트로 옮겨 세로 공간을 편집기에 돌려준다.
 */
function SettingsSheet({ fields, onClose }: { fields: SettingField[]; onClose: () => void }) {
  useEffect(() => {
    const handleKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [onClose])

  return (
    <div className="fixed inset-0 z-40 flex flex-col justify-end md:hidden">
      <button
        type="button"
        aria-label="설정 닫기"
        onClick={onClose}
        className="flex-1 bg-foreground/20"
      />
      <div className="rounded-t-xl border-t bg-background px-4 pt-3 pb-[max(1rem,env(safe-area-inset-bottom))]">
        <div className="flex items-center justify-between pb-2">
          <span className="text-sm font-medium">설정</span>
          <button type="button" aria-label="닫기" onClick={onClose} className="p-1.5">
            <X className="size-4" />
          </button>
        </div>
        {fields.map((field) => (
          <div key={field.id} className="flex items-center justify-between gap-4 py-2">
            <span className="text-sm text-muted-foreground">{field.label}</span>
            <SettingSelect field={field} className="h-10 w-44 text-sm" />
          </div>
        ))}
      </div>
    </div>
  )
}

export function Toolbar() {
  const ready = usePlaygroundStore((state) => state.ready)
  const backends = usePlaygroundStore((state) => state.backends)
  const backendId = usePlaygroundStore((state) => state.backendId)
  const setBackendId = usePlaygroundStore((state) => state.setBackendId)
  const exampleId = usePlaygroundStore((state) => state.exampleId)
  const loadExample = usePlaygroundStore((state) => state.loadExample)
  const highlightThemeId = usePlaygroundStore((state) => state.highlightThemeId)
  const setHighlightTheme = usePlaygroundStore((state) => state.setHighlightTheme)

  const [settingsOpen, setSettingsOpen] = useState(false)

  const fields: SettingField[] = [
    {
      id: 'example',
      label: '예제',
      value: exampleId,
      onValueChange: loadExample,
      options: EXAMPLES.map((example) => ({ id: example.id, label: example.label })),
      width: 'w-36',
    },
    {
      id: 'backend',
      label: '백엔드',
      value: backendId,
      onValueChange: setBackendId,
      disabled: !ready,
      options: backends.map((backend) => ({ id: backend.id, label: backend.label })),
      width: 'w-36',
    },
    {
      id: 'theme',
      label: '테마',
      value: highlightThemeId,
      onValueChange: setHighlightTheme,
      options: THEMES.map((theme) => ({ id: theme.id, label: theme.label })),
      width: 'w-28',
    },
  ]

  return (
    <header className="flex shrink-0 items-center justify-between gap-4 border-b bg-background px-4 py-2.5 md:px-5 md:py-3">
      <div className="flex shrink-0 items-center gap-2">
        <svg
          viewBox="0 0 64 64"
          aria-hidden="true"
          fill="none"
          stroke="#1d7a58"
          strokeWidth={3.5}
          strokeLinecap="round"
          strokeLinejoin="round"
          className="size-5"
        >
          <path d="M22 11h-8v42h8M42 11h8v42h-8" />
          <path d="M32 47c-7-6-7-18 0-27 7 9 7 21 0 27z" fill="#1d7a58" stroke="none" />
        </svg>
        <h1 className="text-sm font-semibold tracking-tight">나무마크 플레이그라운드</h1>
      </div>

      <div className="hidden items-center gap-4 md:flex">
        {fields.map((field) => (
          <div key={field.id} className="flex items-center gap-2">
            <span className="text-xs text-muted-foreground">{field.label}</span>
            <SettingSelect field={field} className={`h-8 ${field.width} text-sm`} />
          </div>
        ))}
      </div>

      <button
        type="button"
        aria-label="설정"
        aria-expanded={settingsOpen}
        onClick={() => setSettingsOpen(true)}
        className="-mr-1.5 p-1.5 text-muted-foreground md:hidden"
      >
        <Settings2 className="size-5" />
      </button>

      {settingsOpen ? (
        <SettingsSheet fields={fields} onClose={() => setSettingsOpen(false)} />
      ) : null}
    </header>
  )
}
