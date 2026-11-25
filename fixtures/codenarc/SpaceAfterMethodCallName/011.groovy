
            // with range in a method call
            someRepository.saveAll( ( 1..3 ).collect { new SomeObjectBuilder().build() } )
            // with calculation in a method call
            someMethod( ( a + b ) / c )
            // operation on result
            someMethod( ( collectionA + collectionB ).toSet() )
        