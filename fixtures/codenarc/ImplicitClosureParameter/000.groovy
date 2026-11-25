
            def validClosure = { parameterName -> def it }
            def validClosureWithMultipleParameters = { first, second -> def it }
            def validClosureWithoutParameters = { -> def it }
            def closureWithNotReferencedImplicitParameter = { println 123 }
            def it
        