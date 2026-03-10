import { AlertCircle, CheckCircle2, Circle, Loader2 } from 'lucide-react';

export type WebviewFlowStepStatus = 'pending' | 'running' | 'success' | 'error';

export interface WebviewFlowStepItem {
  key: string;
  title: string;
  status: WebviewFlowStepStatus;
  description?: string;
  retryLabel?: string;
  onRetry?: () => void;
  retryDisabled?: boolean;
}

interface WebviewFlowStepsProps {
  steps: WebviewFlowStepItem[];
}

function renderStatusIcon(status: WebviewFlowStepStatus) {
  if (status === 'success') {
    return <CheckCircle2 size={15} />;
  }
  if (status === 'running') {
    return <Loader2 size={15} className="loading-spinner" />;
  }
  if (status === 'error') {
    return <AlertCircle size={15} />;
  }
  return <Circle size={15} />;
}

export function WebviewFlowSteps({ steps }: WebviewFlowStepsProps) {
  return (
    <div className="webview-flow-steps">
      {steps.map((step, index) => (
        <div
          key={step.key}
          className={`webview-flow-step webview-flow-step-${step.status}`}
        >
          <div className="webview-flow-step-main">
            <span className="webview-flow-step-index">{index + 1}</span>
            <span className="webview-flow-step-icon">{renderStatusIcon(step.status)}</span>
            <span className="webview-flow-step-title">{step.title}</span>
            {step.retryLabel && step.onRetry && (
              <button
                type="button"
                className="webview-flow-step-retry"
                onClick={step.onRetry}
                disabled={step.retryDisabled}
              >
                {step.retryLabel}
              </button>
            )}
          </div>
          {step.description && (
            <div className="webview-flow-step-desc">{step.description}</div>
          )}
        </div>
      ))}
    </div>
  );
}
