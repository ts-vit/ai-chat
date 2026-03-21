import { IconMessage, IconRobot } from '@tabler/icons-react';

export type AppMode = 'chat' | 'assistant';

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
];

export const DEFAULT_MODE: AppMode = 'chat';
