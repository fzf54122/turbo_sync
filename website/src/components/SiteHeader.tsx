import { navItems } from '../data/siteContent';

const mascotSrc = `${import.meta.env.BASE_URL}feisuo-mascot.svg`;

export function SiteHeader(): JSX.Element {
  return (
    <header className="site-header">
      <a className="brand-lockup" href="#top" aria-label="返回 TurboSync 首页顶部">
        <img src={mascotSrc} width="40" height="40" alt="" />
        <span>
          <strong>飞梭同步</strong>
          <small>TurboSync</small>
        </span>
      </a>

      <nav className="header-nav" aria-label="官网导航">
        {navItems.map((item) => (
          <a key={item.href} href={item.href}>
            {item.label}
          </a>
        ))}
      </nav>

      <a
        className="header-github"
        href="https://github.com/fzf54122/turbo_sync"
        aria-label="在 GitHub 查看 TurboSync 仓库"
      >
        GitHub
      </a>
    </header>
  );
}
