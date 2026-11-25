
            // usage not chained invocation
            writer.append('Hello')
            writer.append('World')

            writer.append(null).append(5)           // nulls cannot be joined

            writer.append().append('Hello')             // no arg append is unknown
            writer.append('a', 'b').append('Hello')     // two arg append is unknown
        