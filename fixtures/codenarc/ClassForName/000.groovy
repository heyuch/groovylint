
            Class.forName() // zero args is not API
            Class.forName(aClassName, true) // two args is not API
            Class.forName(aClassName, true, aClassLoader, unknown)  // four args is not API
        