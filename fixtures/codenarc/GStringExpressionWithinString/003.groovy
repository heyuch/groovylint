
            @SomeAnnotationOnClass(attribute='${sample.property1}',
                            nested=[@NestedAnnotation('${sample.property2}'),
                                    @NestedAnnotation('${sample.property3}')],
                             someOtherAttribute='${sample.property4}')
            class SomeClass {
            }
        