import { FeatureGrid } from './components/FeatureGrid';
import { Hero } from './components/Hero';
import { HowItWorks } from './components/HowItWorks';
import { Positioning } from './components/Positioning';
import { Quickstart } from './components/Quickstart';
import { SiteFooter } from './components/SiteFooter';
import { SiteHeader } from './components/SiteHeader';
import { StabilityNotice } from './components/StabilityNotice';
import { SurfaceShowcase } from './components/SurfaceShowcase';
import { UseCases } from './components/UseCases';

export default function App(): JSX.Element {
  return (
    <div className="site-shell">
      <SiteHeader />
      <main>
        <Hero />
        <Positioning />
        <SurfaceShowcase />
        <FeatureGrid />
        <HowItWorks />
        <UseCases />
        <Quickstart />
        <StabilityNotice />
      </main>
      <SiteFooter />
    </div>
  );
}
