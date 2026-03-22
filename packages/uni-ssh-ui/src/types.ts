export interface SshTunnelStatus {
  connected: boolean;
  localPort: number | null;
  remoteHost: string | null;
}
