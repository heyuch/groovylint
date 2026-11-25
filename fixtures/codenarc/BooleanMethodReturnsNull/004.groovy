
            def x = new Object() {
                def scriptMethod() {
                    if (foo()) {
                        return true
                    } else {
                        return null
                    }
                }
            }
        