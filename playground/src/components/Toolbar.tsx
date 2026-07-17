import { RotateCcw } from 'lucide-react'

import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { usePlaygroundStore } from '@/store'

export function Toolbar() {
  const ready = usePlaygroundStore((state) => state.ready)
  const backends = usePlaygroundStore((state) => state.backends)
  const backendId = usePlaygroundStore((state) => state.backendId)
  const setBackendId = usePlaygroundStore((state) => state.setBackendId)
  const resetSample = usePlaygroundStore((state) => state.resetSample)

  return (
    <header className="flex items-center justify-between gap-4 border-b bg-secondary px-4 py-2.5">
      <h1 className="text-sm font-semibold">나무마크 플레이그라운드</h1>
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
        <Button variant="outline" size="sm" onClick={resetSample}>
          <RotateCcw />
          예제
        </Button>
      </div>
    </header>
  )
}
