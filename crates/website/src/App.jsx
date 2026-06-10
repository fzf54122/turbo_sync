import { useState, useEffect } from 'react'
import './App.css'

const REPO_URL = 'https://github.com/fzf54122/turbo_sync'

function getSystemTheme() {
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function applyTheme(newTheme) {
  const resolvedTheme = newTheme === 'auto' ? getSystemTheme() : newTheme
  document.documentElement.setAttribute('data-theme', resolvedTheme)
}

function App() {
  const [theme, setTheme] = useState('auto')
  const [lang, setLang] = useState('zh')
  const [page, setPage] = useState('home')
  const [showThemeMenu, setShowThemeMenu] = useState(false)
  const [showLangMenu, setShowLangMenu] = useState(false)

  useEffect(() => {
    const savedTheme = localStorage.getItem('theme') || 'auto'
    const savedLang = localStorage.getItem('lang') || 'zh'
    setTheme(savedTheme)
    setLang(savedLang)
    applyTheme(savedTheme)

    const syncPageFromHash = () => {
      const hashPage = window.location.hash.replace('#/', '')
      setPage(['docs', 'download'].includes(hashPage) ? hashPage : 'home')
    }

    syncPageFromHash()
    window.addEventListener('hashchange', syncPageFromHash)

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    const handleSystemThemeChange = () => {
      if ((localStorage.getItem('theme') || 'auto') === 'auto') {
        applyTheme('auto')
      }
    }

    mediaQuery.addEventListener('change', handleSystemThemeChange)
    return () => {
      window.removeEventListener('hashchange', syncPageFromHash)
      mediaQuery.removeEventListener('change', handleSystemThemeChange)
    }
  }, [])

  const navigateTo = (nextPage) => {
    setPage(nextPage)
    window.location.hash = nextPage === 'home' ? '' : `/${nextPage}`
    window.scrollTo({ top: 0, behavior: 'smooth' })
  }

  const handleThemeChange = (newTheme) => {
    setTheme(newTheme)
    localStorage.setItem('theme', newTheme)
    applyTheme(newTheme)
    setShowThemeMenu(false)
  }

  const handleLangChange = (newLang) => {
    setLang(newLang)
    localStorage.setItem('lang', newLang)
    setShowLangMenu(false)
  }

  const t = {
    zh: {
      logo: '飞梭同步',
      features: '功能特性',
      docs: '文档',
      download: '下载',
      github: 'GitHub',
      heroTitle: '飞梭同步',
      heroSubtitle: 'Intelligence, Crafted.',
      heroDesc: '跨平台文件同步工具，适合开发目录、NAS、服务器和 Homelab 场景',
      productTitle: 'TurboSync Agent',
      productDesc: '本机后台同步服务，负责扫描、监听、同步和传输',
      downloadBtn: '立即下载',
      ctaTitle: '探索智能，创造可能',
      ctaDesc: 'Explore intelligence. Create possibility.',
      ctaBtn: '开始使用',
      docsTitle: '飞梭同步文档',
      docsDesc: '在官网了解核心概念、使用流程和常用命令。',
      downloadTitle: '下载飞梭同步',
      downloadDesc: '选择适合日常桌面、服务器或开发场景的入口。',
      sourceCode: '源码仓库',
      releaseDetails: 'Release 详情',
      backHome: '返回首页',
      footer: '© 2024 飞梭同步 · MIT License',
      theme: {
        light: '浅色模式',
        dark: '深色模式',
        auto: '自动模式'
      }
    },
    en: {
      logo: 'TurboSync',
      features: 'Features',
      docs: 'Docs',
      download: 'Download',
      github: 'GitHub',
      heroTitle: 'TurboSync',
      heroSubtitle: 'Intelligence, Crafted.',
      heroDesc: 'Cross-platform file sync tool for development, NAS, servers and Homelab',
      productTitle: 'TurboSync Agent',
      productDesc: 'Local background sync service for scanning, monitoring, syncing and transferring',
      downloadBtn: 'Download Now',
      ctaTitle: 'Explore Intelligence, Create Possibility',
      ctaDesc: 'Explore intelligence. Create possibility.',
      ctaBtn: 'Get Started',
      docsTitle: 'TurboSync Docs',
      docsDesc: 'Learn the concepts, workflow, and common commands on the official site.',
      downloadTitle: 'Download TurboSync',
      downloadDesc: 'Choose the right entry for desktop, server, or developer workflows.',
      sourceCode: 'Source Code',
      releaseDetails: 'Release Details',
      backHome: 'Back Home',
      footer: '© 2024 TurboSync · MIT License',
      theme: {
        light: 'Light',
        dark: 'Dark',
        auto: 'Auto'
      }
    }
  }

  const content = t[lang]

  const features = lang === 'zh' ? [
    { title: '桌面 GUI', desc: 'Tauri + React 桌面控制台' },
    { title: '终端 TUI', desc: '中文默认，支持英文切换' },
    { title: '实时同步', desc: '基于文件系统监听' },
    { title: '增量传输', desc: '只传输变化的部分' },
    { title: 'SQLite 存储', desc: '轻量级本地数据库' },
    { title: '跨平台', desc: 'Linux / macOS / Windows' },
    { title: 'Rust 编写', desc: '高性能、内存安全' },
    { title: '双向同步', desc: '多节点互相同步' },
    { title: '冲突处理', desc: '智能合并冲突文件' },
    { title: '过滤规则', desc: '自定义忽略模式' },
    { title: 'CLI 命令', desc: '命令行完整控制' },
    { title: '开源免费', desc: 'MIT 协议' }
  ] : [
    { title: 'Desktop GUI', desc: 'Tauri + React console' },
    { title: 'Terminal TUI', desc: 'Bilingual support' },
    { title: 'Real-time Sync', desc: 'Filesystem watch' },
    { title: 'Incremental', desc: 'Delta transfer' },
    { title: 'SQLite', desc: 'Lightweight database' },
    { title: 'Cross-platform', desc: 'Linux / macOS / Windows' },
    { title: 'Rust', desc: 'Performance & Safety' },
    { title: 'Bidirectional', desc: 'Multi-node sync' },
    { title: 'Conflict', desc: 'Smart merge' },
    { title: 'Filters', desc: 'Custom ignore patterns' },
    { title: 'CLI', desc: 'Full control' },
    { title: 'Open Source', desc: 'MIT License' }
  ]

  const docs = lang === 'zh' ? [
    { title: '日常使用推荐', desc: '优先使用桌面 GUI 或终端 TUI。它们会自动初始化配置、连接本机 Agent，并提供节点、任务、同步和日志入口。' },
    { title: '后台同步服务', desc: 'turbosync-agent 负责扫描目录、监听文件变化、生成同步计划，并通过本地复制或 QUIC 传输文件。' },
    { title: '同步任务模型', desc: '一个任务由源目录、目标节点和目标路径组成。当前最稳定主线是源目录到目标目录的单向同步。' },
    { title: '服务器与 Homelab', desc: '远端机器只需要运行 Agent，然后在本机添加节点即可。适合 NAS、服务器备份和多设备目录同步。' }
  ] : [
    { title: 'Recommended daily flow', desc: 'Start with the desktop GUI or terminal TUI. They initialize config, connect to the local Agent, and expose nodes, tasks, sync, and logs.' },
    { title: 'Background sync service', desc: 'turbosync-agent scans folders, watches file changes, creates sync plans, and transfers files by local copy or QUIC.' },
    { title: 'Task model', desc: 'A task is defined by a source folder, a target node, and a target path. The most stable path today is source-to-target one-way sync.' },
    { title: 'Servers and Homelab', desc: 'Remote machines only need to run the Agent. Add them as nodes locally for NAS, server backup, and multi-device folder sync.' }
  ]

  const downloads = lang === 'zh' ? [
    { title: '桌面端', desc: '推荐给普通用户。图形化管理节点、任务和状态。', action: '查看桌面端下载' },
    { title: '命令行', desc: '适合服务器、脚本、CI 和自动化部署。', action: '查看 CLI 下载' },
    { title: '源码构建', desc: '适合开发者审计、贡献或二次开发。', action: '打开源码仓库' }
  ] : [
    { title: 'Desktop App', desc: 'Recommended for daily users. Manage nodes, tasks, and status visually.', action: 'View Desktop Download' },
    { title: 'Command Line', desc: 'For servers, scripts, CI, and automation.', action: 'View CLI Download' },
    { title: 'Build from Source', desc: 'For auditing, contribution, or customization.', action: 'Open Source Repo' }
  ]

  return (
    <div className="app">
      <header className="header">
        <button type="button" className="header-left" onClick={() => navigateTo('home')} aria-label={content.logo}>
          <img src={`${import.meta.env.BASE_URL}feisuo-mascot.svg`} alt="" className="mascot" />
          <div className="logo">{content.logo}</div>
        </button>
        <nav className="nav">
          <button type="button" onClick={() => navigateTo('home')}>{content.features}</button>
          <button type="button" onClick={() => navigateTo('docs')}>{content.docs}</button>
          <button type="button" onClick={() => navigateTo('download')}>{content.download}</button>
          <a href={REPO_URL} target="_blank" rel="noopener noreferrer">{content.github}</a>

          <div className="dropdown">
            <button type="button" onClick={() => setShowThemeMenu(!showThemeMenu)} className="icon-btn" aria-label={content.theme.auto}>
              {theme === 'dark' ? '🌙' : theme === 'light' ? '☀️' : '💻'}
            </button>
            {showThemeMenu && (
              <div className="dropdown-menu">
                <button type="button" onClick={() => handleThemeChange('light')}>☀️ {content.theme.light}</button>
                <button type="button" onClick={() => handleThemeChange('dark')}>🌙 {content.theme.dark}</button>
                <button type="button" onClick={() => handleThemeChange('auto')}>💻 {content.theme.auto}</button>
              </div>
            )}
          </div>

          <div className="dropdown">
            <button type="button" onClick={() => setShowLangMenu(!showLangMenu)} className="icon-btn" aria-label="Language">
              {lang === 'zh' ? '🇨🇳' : '🇬🇧'}
            </button>
            {showLangMenu && (
              <div className="dropdown-menu">
                <button type="button" onClick={() => handleLangChange('zh')}>🇨🇳 简体中文</button>
                <button type="button" onClick={() => handleLangChange('en')}>🇬🇧 English</button>
              </div>
            )}
          </div>
        </nav>
      </header>

      <main>
        {page === 'home' && (
          <>
            <section className="hero">
              <img src={`${import.meta.env.BASE_URL}feisuo-mascot.svg`} alt="飞梭吉祥物" className="hero-mascot" />
              <h1 className="hero-title">{content.heroTitle}</h1>
              <p className="hero-subtitle">{content.heroSubtitle}</p>
              <p className="hero-desc">{content.heroDesc}</p>
            </section>

            <section id="features" className="features" aria-label={content.features}>
              {features.map((feature) => (
                <div key={feature.title} className="feature-card">
                  <h3>{feature.title}</h3>
                  <p>{feature.desc}</p>
                </div>
              ))}
            </section>
          </>
        )}

        {page === 'docs' && (
          <section className="page-section">
            <div className="page-heading">
              <p className="eyebrow">Docs</p>
              <h1>{content.docsTitle}</h1>
              <p>{content.docsDesc}</p>
            </div>
            <div className="doc-grid">
              {docs.map((doc) => (
                <article key={doc.title} className="doc-card">
                  <h3>{doc.title}</h3>
                  <p>{doc.desc}</p>
                </article>
              ))}
            </div>
            <div className="code-block">
              <div className="code-line"><span>1</span><code>tsync init</code></div>
              <div className="code-line"><span>2</span><code>tsync agent run</code></div>
              <div className="code-line"><span>3</span><code>tsync dashboard</code></div>
            </div>
          </section>
        )}

        {page === 'download' && (
          <section className="page-section">
            <div className="page-heading">
              <p className="eyebrow">Download</p>
              <h1>{content.downloadTitle}</h1>
              <p>{content.downloadDesc}</p>
            </div>
            <div className="download-grid">
              {downloads.map((item, index) => (
                <article key={item.title} className="download-card">
                  <h3>{item.title}</h3>
                  <p>{item.desc}</p>
                  <a href={index === 2 ? REPO_URL : `${REPO_URL}/releases`} target="_blank" rel="noopener noreferrer">
                    {item.action}
                  </a>
                </article>
              ))}
            </div>
          </section>
        )}
      </main>

      <footer className="footer">
        <p>{content.footer}</p>
      </footer>
    </div>
  )
}

export default App
