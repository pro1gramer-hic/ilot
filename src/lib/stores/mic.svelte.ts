// Who has the microphone: the chat's dictation and « OK Mochi » use the same speech
// recognizer of Windows, so « OK Mochi » steps aside while the dictation listens. Kept
// apart from both stores so that neither has to import the other.

class Mic {
  /** The chat's dictation listens now. */
  dictating = $state(false);
}

export const mic = new Mic();
