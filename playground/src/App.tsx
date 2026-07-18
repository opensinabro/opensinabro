import { useEffect } from 'react'

import { Toolbar } from '@/components/Toolbar'
import { HighlightedEditor } from '@/components/HighlightedEditor'
import { Preview } from '@/components/Preview'
import { useIsMobile } from '@/lib/useIsMobile'
import { cn } from '@/lib/utils'
import { usePlaygroundStore, type MobilePane } from '@/store'

const PANE_LABELS: { id: MobilePane; label: string }[] = [
  { id: 'editor', label: '편집' },
  { id: 'preview', label: '미리보기' },
]

export function App() {
  const init = usePlaygroundStore((state) => state.init)
  const render = usePlaygroundStore((state) => state.render)
  const source = usePlaygroundStore((state) => state.source)
  const backendId = usePlaygroundStore((state) => state.backendId)
  const output = usePlaygroundStore((state) => state.output)
  const error = usePlaygroundStore((state) => state.error)
  const mobilePane = usePlaygroundStore((state) => state.mobilePane)
  const setMobilePane = usePlaygroundStore((state) => state.setMobilePane)

  const isMobile = useIsMobile()

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
        <section
          className={cn('min-h-0 md:block', isMobile && mobilePane !== 'editor' && 'hidden')}
        >
          <HighlightedEditor />
        </section>
        <section
          className={cn(
            'min-h-0 overflow-hidden md:block',
            isMobile && mobilePane !== 'preview' && 'hidden',
          )}
        >
          {error ? (
            <pre className="overflow-auto p-4 font-mono text-sm break-words whitespace-pre-wrap text-destructive">
              {error}
            </pre>
          ) : (
            <div className="h-full overflow-auto px-4 py-4 md:px-6">
              <Preview output={output} />
            </div>
          )}
        </section>
      </main>
      {/* 전환 바는 엄지가 닿는 하단에 둔다 — 홈 인디케이터 영역만큼 안쪽 여백을 준다. */}
      <nav className="flex shrink-0 gap-1 border-t bg-background p-1 pb-[max(0.25rem,env(safe-area-inset-bottom))] md:hidden">
        {PANE_LABELS.map((pane) => (
          <button
            key={pane.id}
            type="button"
            onClick={() => setMobilePane(pane.id)}
            aria-pressed={mobilePane === pane.id}
            className={cn(
              'flex-1 rounded-md px-3 py-2.5 text-sm font-medium transition-colors',
              mobilePane === pane.id
                ? 'bg-secondary text-secondary-foreground'
                : 'text-muted-foreground',
            )}
          >
            {pane.label}
          </button>
        ))}
      </nav>
    </div>
  )
}
