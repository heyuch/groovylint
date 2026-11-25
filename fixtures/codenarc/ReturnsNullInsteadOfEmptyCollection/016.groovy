
            def a = {
                def b = {
                    if (x) return null      // bad
                    return []
                }
                def c = {
                    if (x) return null      // bad
                    return []
                }
                return []  // ok
            }
        