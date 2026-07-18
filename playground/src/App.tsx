import { useEffect } from 'react'

import { Toolbar } from '@/components/Toolbar'
import { HighlightedEditor } from '@/components/HighlightedEditor'
import { Preview } from '@/components/Preview'
import { usePlaygroundStore } from '@/store'

export function App() {
  const init = usePlaygroundStore((state) => state.init)
  const render = usePlaygroundStore((state) => state.render)
  const source = usePlaygroundStore((state) => state.source)
  const backendId = usePlaygroundStore((state) => state.backendId)
  const output = usePlaygroundStore((state) => state.output)
  const error = usePlaygroundStore((state) => state.error)

  useEffect(() => {
    void init()
  }, [init])

  useEffect(() => {
    const timer = setTimeout(render, 120)
    return () => clearTimeout(timer)
  }, [source, backendId, render])

  return (
    <div className="flex h-full flex-col">
      <Toolbar />
      <main className="grid min-h-0 flex-1 grid-cols-1 md:grid-cols-2">
        <HighlightedEditor />
        <section className="min-h-0 overflow-hidden">
          {error ? (
            <pre className="overflow-auto p-4 font-mono text-sm text-destructive">{error}</pre>
          ) : (
            <div className="h-full overflow-auto px-6 py-4">
              <Preview output={output} />
            </div>
          )}
        </section>
      </main>
    </div>
  )
}
