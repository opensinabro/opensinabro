import { Textarea } from '@/components/ui/textarea'
import { usePlaygroundStore } from '@/store'

export function Editor() {
  const source = usePlaygroundStore((state) => state.source)
  const setSource = usePlaygroundStore((state) => state.setSource)

  return (
    <Textarea
      value={source}
      onChange={(event) => setSource(event.target.value)}
      spellCheck={false}
      className="h-full w-full resize-none rounded-none border-0 border-r px-4 py-3 font-mono text-sm leading-relaxed shadow-none focus-visible:ring-0"
    />
  )
}
