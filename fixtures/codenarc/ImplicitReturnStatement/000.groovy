
            class MyClass {
                String baseName = "base"
                String getName(String id) {
                    return baseName + "." + id
                }

                boolean example() { return true }

                boolean longerExample() {
                    if (baseName == null) {
                        return true
                    }
                    if (baseName == 'abc' && System.getProperty('test')) {
                        println 'abc'
                        return true
                    }
                    return false
                }
            }
        