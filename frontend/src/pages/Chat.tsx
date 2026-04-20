import { useRef, useState } from 'react'
import { ArrowUp, Bot, Cpu, RotateCcw, User } from 'lucide-react'
import { sendAsk } from '../lib/api'
import { cn, formatRelative } from '../lib/utils'

interface Message {
  role: 'user' | 'assistant'
  content: string
  at: string
  loading?: boolean
}

export function Chat() {
  const [messages, setMessages] = useState<Message[]>([])
  const [input, setInput] = useState('')
  const [loading, setLoading] = useState(false)
  const bottomRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  const scrollToBottom = () => {
    setTimeout(() => bottomRef.current?.scrollIntoView({ behavior: 'smooth' }), 50)
  }

  const send = async () => {
    const prompt = input.trim()
    if (!prompt || loading) return

    const userMsg: Message = { role: 'user', content: prompt, at: new Date().toISOString() }
    const pendingMsg: Message = {
      role: 'assistant',
      content: '',
      at: new Date().toISOString(),
      loading: true,
    }

    setMessages((prev) => [...prev, userMsg, pendingMsg])
    setInput('')
    setLoading(true)
    scrollToBottom()

    try {
      const res = await sendAsk(prompt)
      const output = res.output ?? ''
      setMessages((prev) => {
        const next = [...prev]
        next[next.length - 1] = {
          role: 'assistant',
          content: output,
          at: new Date().toISOString(),
        }
        return next
      })
    } catch (err) {
      setMessages((prev) => {
        const next = [...prev]
        next[next.length - 1] = {
          role: 'assistant',
          content: `Error: ${err instanceof Error ? err.message : String(err)}`,
          at: new Date().toISOString(),
        }
        return next
      })
    } finally {
      setLoading(false)
      scrollToBottom()
    }
  }

  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      send()
    }
  }

  const clear = () => setMessages([])

  return (
    <div className="flex flex-col h-[calc(100vh-7rem)]">
      <div className="page-header flex-shrink-0">
        <div>
          <h1 className="page-title">Chat</h1>
          <p className="page-subtitle">Talk to Praxis — stateless ask mode</p>
        </div>
        {messages.length > 0 && (
          <button onClick={clear} className="btn-ghost">
            <RotateCcw className="w-3.5 h-3.5" />
            Clear
          </button>
        )}
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto space-y-4 pb-4">
        {messages.length === 0 && (
          <div className="flex flex-col items-center justify-center h-full text-center px-8">
            <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-brand-500 to-violet-500 flex items-center justify-center mb-4 shadow-lg shadow-brand-500/20">
              <Cpu className="w-8 h-8 text-white" />
            </div>
            <h2 className="text-lg font-semibold text-slate-800 dark:text-slate-200">
              Ask Praxis anything
            </h2>
            <p className="mt-2 text-sm text-slate-500 dark:text-slate-400 max-w-sm">
              Stateless ask — no session state is created or mutated. Fast responses, direct
              answers.
            </p>
            <div className="mt-6 grid grid-cols-1 sm:grid-cols-2 gap-2 w-full max-w-md">
              {[
                'What are my active goals?',
                'Summarize the last session',
                'What tools are registered?',
                'Show the current identity',
              ].map((s) => (
                <button
                  key={s}
                  onClick={() => {
                    setInput(s)
                    textareaRef.current?.focus()
                  }}
                  className="text-left px-4 py-3 text-sm rounded-xl border border-slate-200 dark:border-slate-700
                             bg-white dark:bg-slate-900 hover:border-brand-300 dark:hover:border-brand-700
                             text-slate-600 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-400
                             transition-all"
                >
                  {s}
                </button>
              ))}
            </div>
          </div>
        )}

        {messages.map((msg, i) => (
          <div
            key={i}
            className={cn(
              'flex gap-3',
              msg.role === 'user' ? 'flex-row-reverse' : 'flex-row',
            )}
          >
            {/* Avatar */}
            <div
              className={cn(
                'flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center text-white text-xs font-bold',
                msg.role === 'user'
                  ? 'bg-brand-500'
                  : 'bg-gradient-to-br from-violet-500 to-brand-500',
              )}
            >
              {msg.role === 'user' ? <User className="w-4 h-4" /> : <Bot className="w-4 h-4" />}
            </div>

            <div
              className={cn(
                'max-w-[85%] rounded-2xl px-4 py-3',
                msg.role === 'user'
                  ? 'bg-brand-600 text-white rounded-tr-sm'
                  : 'bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 text-slate-800 dark:text-slate-200 rounded-tl-sm',
              )}
            >
              {msg.loading ? (
                <div className="flex gap-1 py-1">
                  {[0, 1, 2].map((j) => (
                    <div
                      key={j}
                      className="w-2 h-2 rounded-full bg-slate-400 animate-bounce"
                      style={{ animationDelay: `${j * 0.15}s` }}
                    />
                  ))}
                </div>
              ) : (
                <pre className="whitespace-pre-wrap text-sm leading-relaxed font-sans break-words">
                  {msg.content}
                </pre>
              )}
              <p
                className={cn(
                  'mt-1.5 text-xs',
                  msg.role === 'user' ? 'text-brand-200' : 'text-slate-400',
                )}
              >
                {formatRelative(msg.at)}
              </p>
            </div>
          </div>
        ))}
        <div ref={bottomRef} />
      </div>

      {/* Input */}
      <div className="flex-shrink-0 card p-3 flex items-end gap-3">
        <textarea
          ref={textareaRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder="Ask Praxis something… (Enter to send, Shift+Enter for newline)"
          rows={2}
          disabled={loading}
          className="flex-1 input-base resize-none min-h-[2.5rem] max-h-40"
        />
        <button
          onClick={send}
          disabled={loading || !input.trim()}
          className="btn-primary h-10 w-10 p-0 rounded-xl justify-center"
        >
          <ArrowUp className="w-4 h-4" />
        </button>
      </div>
    </div>
  )
}
