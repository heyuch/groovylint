
            class MyClass {
                String baseName = "base"
                String getName(String id) {
                    baseName + "." + id
                }

                boolean example() { true }

                protected int longerExample() {
                    if (baseName == null) {
                        return 0
                    }
                    if (baseName == 'abc' && System.getProperty('test')) {
                        println 'abc'
                        return 3
                    }
                    99
                }

                private Object emptyMethod() { }
            }
        