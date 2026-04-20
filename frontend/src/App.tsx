import { Routes, Route, Navigate } from 'react-router-dom'
import { Layout } from './components/layout/Layout'
import { Dashboard } from './pages/Dashboard'
import { Chat } from './pages/Chat'
import { Sessions } from './pages/Sessions'
import { Goals } from './pages/Goals'
import { Memories } from './pages/Memories'
import { Identity } from './pages/Identity'
import { Tools } from './pages/Tools'
import { Approvals } from './pages/Approvals'
import { Evolution } from './pages/Evolution'
import { Score } from './pages/Score'
import { Learning } from './pages/Learning'
import { Canary } from './pages/Canary'
import { Config } from './pages/Config'
import { Boundaries } from './pages/Boundaries'
import { Delegation } from './pages/Delegation'
import { Forensics } from './pages/Forensics'
import { Agents } from './pages/Agents'
import { Vault } from './pages/Vault'
import { Argus } from './pages/Argus'

export default function App(): React.ReactElement {
  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        <Route index element={<Navigate to="/dashboard" replace />} />
        <Route path="dashboard" element={<Dashboard />} />
        <Route path="chat" element={<Chat />} />
        <Route path="sessions" element={<Sessions />} />
        <Route path="goals" element={<Goals />} />
        <Route path="memories" element={<Memories />} />
        <Route path="identity" element={<Identity />} />
        <Route path="tools" element={<Tools />} />
        <Route path="approvals" element={<Approvals />} />
        <Route path="evolution" element={<Evolution />} />
        <Route path="score" element={<Score />} />
        <Route path="learning" element={<Learning />} />
        <Route path="canary" element={<Canary />} />
        <Route path="config" element={<Config />} />
        <Route path="boundaries" element={<Boundaries />} />
        <Route path="delegation" element={<Delegation />} />
        <Route path="forensics" element={<Forensics />} />
        <Route path="agents" element={<Agents />} />
        <Route path="vault" element={<Vault />} />
        <Route path="argus" element={<Argus />} />
        <Route path="*" element={<Navigate to="/dashboard" replace />} />
      </Route>
    </Routes>
  )
}
