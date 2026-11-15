import React from 'react';

type Props = {
  command: string;
  children?: React.ReactNode;
};

export default function CommandStep({command, children}: Props): React.ReactElement {
  return (
    <li className="step step--command">
      {children ? <div className="step__text">{children}</div> : null}
      <pre>
        <code>{command}</code>
      </pre>
    </li>
  );
}
