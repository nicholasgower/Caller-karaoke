import React from 'react';
import Shot from '@site/src/components/Shot';

type Props = {
  shot: string;
  children: React.ReactNode;
};

export default function Step({shot, children}: Props): React.ReactElement {
  return (
    <li className="step">
      <div className="step__text">{children}</div>
      <Shot id={shot} />
    </li>
  );
}
