const comparisons = [
  ['rsync', '足够强大，但参数与使用模型对普通用户不够友好。'],
  ['Syncthing', '功能完整，但第一次配置需要理解较多同步概念。'],
  ['云盘产品', '方便但不适合所有本地、NAS、服务器和 Homelab 场景。'],
  ['自写脚本', '能救急，但长期维护、错误恢复和可观测性都容易变成负担。'],
];

export function Positioning(): JSX.Element {
  return (
    <section className="content-section positioning-section" id="why" aria-labelledby="why-title">
      <div className="section-kicker">WHY TURBOSYNC</div>
      <div className="split-layout">
        <div>
          <h2 id="why-title">不是另一个复杂云盘，而是一条更少概念的同步链路。</h2>
          <p>
            TurboSync 关注一个更小的问题：让个人开发者、NAS 用户、Linux 用户和 Homelab 用户，用更少配置完成稳定的文件同步。
          </p>
        </div>
        <div className="comparison-stack">
          {comparisons.map(([name, text]) => (
            <article key={name} className="comparison-card">
              <span>{name}</span>
              <p>{text}</p>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}
