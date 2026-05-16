'use client';

import { Drawer } from 'antd';

import DeveloperProfile from './developer-profile';

// Mock constants since files are missing
const isDesktop = true;
const TITLE_BAR_HEIGHT = 0;

interface DeveloperPanelProps {
  onClose: () => void;
  open: boolean;
}

export default function DeveloperPanel({ open, onClose }: DeveloperPanelProps) {
  return (
    <Drawer
      closable={false}
      destroyOnClose
      onClose={onClose}
      open={open}
      placement={'bottom'}
      size={isDesktop ? 'large' : 'default'}
      styles={{ body: { padding: 0 } }}
    >
      <DeveloperProfile />
    </Drawer>
  );
}
