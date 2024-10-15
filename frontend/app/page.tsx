'use client'

import { useEffect, useState } from 'react';
import { Rocket } from 'lucide-react';

export default function Home() {
  const [sshxUrl, setSshxUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchSshxUrl = async () => {
      try {
        const response = await fetch('/api/sshx');
        if (!response.ok) {
          throw new Error('Failed to fetch SSHX URL');
        }
        const data = await response.json();
        setSshxUrl(data.url);
      } catch (err) {
        setError('Failed to connect. Please try again later.');
      } finally {
        setLoading(false);
      }
    };

    fetchSshxUrl();
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen">
        <Rocket className="w-16 h-16 animate-bounce text-primary" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-screen">
        <p className="text-destructive">{error}</p>
      </div>
    );
  }

  if (sshxUrl) {
    return (
      <iframe src={sshxUrl} className="w-full h-screen border-none" />
    );
  }

  return null;
}
