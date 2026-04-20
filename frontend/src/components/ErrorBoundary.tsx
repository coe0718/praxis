import React from 'react'

interface State {
  error: Error | null
}

export class ErrorBoundary extends React.Component<{ children: React.ReactNode }, State> {
  state: State = { error: null }

  static getDerivedStateFromError(error: Error): State {
    return { error }
  }

  render(): React.ReactNode {
    if (this.state.error) {
      return (
        <div className="min-h-screen flex items-center justify-center bg-slate-50 dark:bg-slate-950">
          <div className="text-center space-y-4 p-8 max-w-md">
            <h1 className="text-xl font-semibold text-red-600 dark:text-red-400">
              Something went wrong
            </h1>
            <p className="text-sm text-slate-500 dark:text-slate-400 font-mono break-all">
              {this.state.error.message}
            </p>
            <button
              onClick={() => this.setState({ error: null })}
              className="btn-primary"
            >
              Retry
            </button>
          </div>
        </div>
      )
    }
    return this.props.children
  }
}
