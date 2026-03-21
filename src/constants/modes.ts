import { IconMessage, IconRobot, IconNotebook } from '@tabler/icons-react';

export type AppMode = 'chat' | 'assistant' | 'notebook';

export interface ModeDefinition {
  id: AppMode;
  icon: React.FC<any>;
  labelKey: string;
  descriptionKey: string;
  color: string;
  defaultEnabled: boolean;
}

export const MODE_DEFINITIONS: ModeDefinition[] = [
  {
    id: 'chat',
    icon: IconMessage,
    labelKey: 'modes.chat',
    descriptionKey: 'modes.chatDescription',
    color: 'brand',
    defaultEnabled: true,
  },
  {
    id: 'assistant',
    icon: IconRobot,
    labelKey: 'modes.assistant',
    descriptionKey: 'modes.assistantDescription',
    color: 'teal',
    defaultEnabled: true,
  },
  {
    id: 'notebook',
    icon: IconNotebook,
    labelKey: 'modes.notebook',
    descriptionKey: 'modes.notebookDescription',
    color: 'violet',
    defaultEnabled: true,
  },
];

export const DEFAULT_MODE: AppMode = 'chat';
