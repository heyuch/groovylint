
            def a =  {
                if (x) return null
                return foo ?: []
            }
            def b =  {
                if (x) return []
                return foo ?: null
            }
        