
            class MyClass {
                def doStuff() {
                    if (name != null || name.size() > 0) { }
                    if (string == null && string.equals("")) { }
                    def isValid = name != null || name.size() > 0
                    return name != null || !name.size()
                }
            }
        