
            class TrailingWhitespaceInMethodCallWithParentheses {
                void invalid() {
                    aMethod ("arg")
                }

                void aMethod(String argument) {
                }
            }
        