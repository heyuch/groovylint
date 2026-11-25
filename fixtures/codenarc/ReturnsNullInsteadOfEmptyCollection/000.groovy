
            List myMethod() {

                def c = {
                    return null // ignore returns from nested closure
                }
                def o = new Object() {
                    def foo() {
                        return null // ignore returns from nested class
                    }
                }
                return []
            }

            def c =  {
                return []
            }
        