
            class ClassUsingMacros {
                Statement statementCreatedUsingMacros() {
                    def code = macro { return toString() } as Statement
                }
            }
        