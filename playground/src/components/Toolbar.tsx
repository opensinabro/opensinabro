import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { EXAMPLES } from '@/examples'
import { usePlaygroundStore } from '@/store'

export function Toolbar() {
  const ready = usePlaygroundStore((state) => state.ready)
  const backends = usePlaygroundStore((state) => state.backends)
  const backendId = usePlaygroundStore((state) => state.backendId)
  const setBackendId = usePlaygroundStore((state) => state.setBackendId)
  const exampleId = usePlaygroundStore((state) => state.exampleId)
  const loadExample = usePlaygroundStore((state) => state.loadExample)
  const mode = usePlaygroundStore((state) => state.mode)
  const setMode = usePlaygroundStore((state) => state.setMode)

  return (
    <header className="flex items-center justify-between gap-4 border-b bg-background px-5 py-3">
      <h1 className="text-sm font-semibold tracking-tight">나무마크 플레이그라운드</h1>
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">예제</span>
          <Select value={exampleId} onValueChange={loadExample}>
            <SelectTrigger className="h-8 w-36 text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {EXAMPLES.map((example) => (
                <SelectItem key={example.id} value={example.id}>
                  {example.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">백엔드</span>
          <Select value={backendId} onValueChange={setBackendId} disabled={!ready}>
            <SelectTrigger className="h-8 w-36 text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {backends.map((backend) => (
                <SelectItem key={backend.id} value={backend.id}>
                  {backend.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className="flex rounded-lg bg-muted p-0.5">
          {(['preview', 'tokens'] as const).map((value) => (
            <button
              key={value}
              type="button"
              onClick={() => setMode(value)}
              className={
                'rounded-md px-3 py-1 text-xs transition-colors ' +
                (mode === value
                  ? 'bg-background font-medium text-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground')
              }
            >
              {value === 'preview' ? '미리보기' : '토큰'}
            </button>
          ))}
        </div>
      </div>
    </header>
  )
}
