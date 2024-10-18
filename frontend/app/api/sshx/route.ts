import { NextRequest, NextResponse } from 'next/server';
import { promises as fs } from 'fs';

// Names are from the host perspective.
const PIPE_WRITER_PATH = '/tmp/sshx-host-runner-read';
const PIPE_READER_PATH = '/tmp/sshx-host-runner-write';

export async function POST(req: NextRequest) {

    try {
        const data = await req.text();
        console.log('Request:', data);
        if (['OpenNewConnection', 'KeepAlive'].indexOf(data) === -1) {
            throw new Error('Invalid request');
        }
        console.log('Reading file from:', PIPE_READER_PATH);
        const responsePromise = fs.readFile(PIPE_READER_PATH, 'utf-8');
        await fs.writeFile(PIPE_WRITER_PATH, data);

        const response = await responsePromise;
        console.log('Response:', response);


        if (response.startsWith('http')) {
            return NextResponse.json({ url: response.trim() });
        } else {
            throw new Error('Invalid response from host process');
        }
    } catch (error) {
        console.error('Failed to communicate with host process:', error);
        return NextResponse.json({ error: 'Failed to communicate with host process' }, { status: 500 });
    }
}
