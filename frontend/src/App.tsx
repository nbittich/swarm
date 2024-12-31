import './App.css'

import AuthProvider from './auth/AuthContext'
import SwarmBrowserRouter from './routes/Router'

function App() {

  return (
    <AuthProvider>
      <SwarmBrowserRouter />
    </AuthProvider>
  )
}

export default App
