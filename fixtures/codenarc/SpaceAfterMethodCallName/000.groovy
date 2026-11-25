
            class Valid {
                Valid() {
                    super()
                }

                void valid() {
                    aMethod()
                    aMethod("arg")
                    aMethod("arg")
                    aMethod "arg"
                    new String(
                        "valid"
                    )
                }

                void aMethod(String argument) {
                }

                def aMethodWithSpecialRegexCharacter() {
                    withFormat {
                        json { }
                        '*'  { }
                    }
                }

                LinkedHashSet<String> set() {
                    new LinkedHashSet<Class<?>>()
                }
            }
        