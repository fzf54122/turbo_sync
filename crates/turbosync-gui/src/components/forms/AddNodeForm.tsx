import { FormEvent, useState } from 'react';
import type { CreateNodeRequest } from '../../types/turbosync';

interface AddNodeFormProps {
  busy: boolean;
  onSubmit: (request: CreateNodeRequest) => Promise<void>;
}

export function AddNodeForm({ busy, onSubmit }: AddNodeFormProps) {
  const [name, setName] = useState('');
  const [endpoint, setEndpoint] = useState('');
  const [fingerprint, setFingerprint] = useState('');

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    await onSubmit({
      name: name.trim(),
      endpoint: endpoint.trim(),
      public_key: fingerprint.trim() || null,
    });
    setName('');
    setEndpoint('');
    setFingerprint('');
  }

  return (
    <form className="space-y-3" onSubmit={handleSubmit}>
      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
        <input className="field" placeholder="节点名称" value={name} onChange={(event) => setName(event.target.value)} required />
        <input className="field" placeholder="127.0.0.1:38746" value={endpoint} onChange={(event) => setEndpoint(event.target.value)} required />
      </div>
      <input className="field" placeholder="证书指纹，可选" value={fingerprint} onChange={(event) => setFingerprint(event.target.value)} />
      <button className="primary-button w-full" disabled={busy || !name.trim() || !endpoint.trim()} type="submit">
        添加同步节点
      </button>
    </form>
  );
}
