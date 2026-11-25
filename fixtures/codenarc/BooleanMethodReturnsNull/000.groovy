
            def c = {
                    if (foo()) {
                        return true
                    } else {
                        return false
                    }
            }
            def scriptMethod() {
                if (foo()) {
                    return true
                } else {
                    return false
                }
            }
            class MyClass {
                def x = new Object() {
                    def method1() {
                        if (foo()) {
                            return true
                        } else {
                            return false
                        }
                    }
                }
                def method2() {
                    def y = {
                        if (foo()) {
                            return true
                        } else {
                            return false
                        }
                    }
                    if (foo()) {
                        return true
                    } else {
                        return false
                    }
                }
                boolean method3() {
                    if (foo()) {
                        return true
                    } else {
                        return false
                    }
                }
                boolean method4() {
                    if (foo()) {
                        return ret() as Boolean
                    } else {
                        return ret() as Boolean
                    }
                }
            }
        