import type { CodebuddyAccount } from '../../types/codebuddy-suite';
import { CodeBuddyCNCheckinModal as SharedCodeBuddyCNCheckinModal } from '../codebuddy-suite/CodebuddySuiteCheckinModal';

interface CodeBuddyCNCheckinModalProps {
  accounts: CodebuddyAccount[];
  onClose: () => void;
  onCheckinComplete?: () => void;
}

export function CodeBuddyCNCheckinModal(props: CodeBuddyCNCheckinModalProps) {
  return <SharedCodeBuddyCNCheckinModal {...props} />;
}
