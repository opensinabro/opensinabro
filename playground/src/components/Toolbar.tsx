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

  return (
    <header className="flex items-center justify-between gap-4 border-b bg-secondary px-4 py-2.5">
      <h1 className="text-sm font-semibold">나무마크 플레이그라운드</h1>
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
      </div>
    </header>
  )
}
