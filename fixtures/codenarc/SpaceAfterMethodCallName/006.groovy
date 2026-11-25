
            class ExcessiveTrailingWhitespaceInMethodCallWithoutParentheses {
                void invalid() {
                    aMethod  "arg"
                }

                void aMethod(String argument) {
                }
            }
        