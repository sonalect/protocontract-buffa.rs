//! The tokens one method contributes to each generated item.

use buffa_codegen::generated::descriptor::MethodDescriptorProto;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// How a method streams.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Unary,
    ServerStreaming,
    ClientStreaming,
    Bidirectional,
}

impl Kind {
    pub(crate) fn of(method: &MethodDescriptorProto) -> Kind {
        match (
            method.client_streaming.unwrap_or(false),
            method.server_streaming.unwrap_or(false),
        ) {
            (false, false) => Kind::Unary,
            (false, true) => Kind::ServerStreaming,
            (true, false) => Kind::ClientStreaming,
            (true, true) => Kind::Bidirectional,
        }
    }

    pub(crate) fn streams_in(self) -> bool {
        matches!(self, Kind::ClientStreaming | Kind::Bidirectional)
    }

    fn streams_out(self) -> bool {
        matches!(self, Kind::ServerStreaming | Kind::Bidirectional)
    }
}

/// One method, as the emitters need it.
pub(crate) struct Method<'a> {
    pub(crate) kind: Kind,
    pub(crate) doc: TokenStream,
    pub(crate) ident: Ident,
    /// The parameter: the request, or the inbound stream.
    pub(crate) parameter: Ident,
    pub(crate) request: TokenStream,
    pub(crate) reply: TokenStream,
    /// The runtime crate path.
    pub(crate) rt: &'a TokenStream,
    /// The service's `<S>Sync` trait.
    pub(crate) sync_trait: &'a Ident,
    /// The service's `<S>Async` trait.
    pub(crate) async_trait: &'a Ident,
}

/// What an async reply stream may capture, in an impl of the async trait.
enum Capture {
    /// The trait's own declaration: the implementing type.
    Trait,
    /// An impl whose reply stream captures nothing of the implementer.
    Nothing,
    /// An impl for a bridge over `T`.
    BridgeParam,
}

impl Method<'_> {
    fn reply_result(&self) -> TokenStream {
        let (reply, rt) = (&self.reply, self.rt);
        quote!(::core::result::Result<#reply, #rt::Error>)
    }

    fn request_item(&self) -> TokenStream {
        let (request, rt) = (&self.request, self.rt);
        quote!(::core::result::Result<#request, #rt::Error>)
    }

    /// `where R: Stream<Item = Result<Req, Error>> + Send + 'static`, for
    /// methods whose async form takes an inbound stream.
    fn inbound_bound(&self) -> TokenStream {
        if !self.kind.streams_in() {
            return TokenStream::new();
        }
        let (rt, item) = (self.rt, self.request_item());
        quote! {
            where R: #rt::Stream<Item = #item> + ::core::marker::Send + 'static
        }
    }

    /// The signature of the method in the sync trait.
    fn sync_signature(&self) -> TokenStream {
        let (ident, request, rt) = (&self.ident, &self.request, self.rt);
        let parameter = &self.parameter;
        let (reply, item) = (self.reply_result(), self.request_item());
        let input = if self.kind.streams_in() {
            quote!(#parameter: #rt::BoxIter<'static, #item>)
        } else {
            quote!(#parameter: #request)
        };
        let output = if self.kind.streams_out() {
            quote!(::core::result::Result<#rt::BoxIter<'static, #reply>, #rt::Error>)
        } else {
            reply
        };
        quote!(fn #ident(&self, #input) -> #output)
    }

    /// The signature of the method in the async trait or an impl of it.
    fn async_signature(&self, capture: Capture) -> TokenStream {
        self.async_signature_as(capture, false)
    }

    /// The signature of the method in the async trait or an impl of it, as
    /// `fn … -> impl Future<Output = O>` or, with `as_async_fn`, as
    /// `async fn … -> O`.
    fn async_signature_as(&self, capture: Capture, as_async_fn: bool) -> TokenStream {
        let (ident, request, rt) = (&self.ident, &self.request, self.rt);
        let parameter = &self.parameter;
        let reply = self.reply_result();
        let send = quote!(::core::marker::Send);
        let (generics, input) = if self.kind.streams_in() {
            (quote!(<R>), quote!(#parameter: R))
        } else {
            (TokenStream::new(), quote!(#parameter: #request))
        };
        let output = if self.kind.streams_out() {
            let captured = match (capture, self.kind.streams_in()) {
                (Capture::Trait, false) => quote!(use<Self>),
                (Capture::Trait, true) => quote!(use<Self, R>),
                (Capture::Nothing, false) => quote!(use<>),
                (Capture::Nothing, true) => quote!(use<R>),
                (Capture::BridgeParam, false) => quote!(use<T>),
                (Capture::BridgeParam, true) => quote!(use<T, R>),
            };
            quote! {
                ::core::result::Result<
                    impl #rt::Stream<Item = #reply> + #send + #captured,
                    #rt::Error,
                >
            }
        } else {
            reply
        };
        let bound = self.inbound_bound();
        if as_async_fn {
            quote!(async fn #ident #generics(&self, #input) -> #output #bound)
        } else {
            quote! {
                fn #ident #generics(&self, #input) -> impl ::core::future::Future<Output = #output> + #send
                #bound
            }
        }
    }

    /// The signature of the method in the dyn-compatible erased trait.
    fn erased_signature(&self) -> TokenStream {
        let (ident, request, rt) = (&self.ident, &self.request, self.rt);
        let parameter = &self.parameter;
        let (reply, item) = (self.reply_result(), self.request_item());
        let input = if self.kind.streams_in() {
            quote!(#parameter: #rt::BoxStream<'static, #item>)
        } else {
            quote!(#parameter: #request)
        };
        let output = if self.kind.streams_out() {
            quote!(::core::result::Result<#rt::BoxStream<'static, #reply>, #rt::Error>)
        } else {
            reply
        };
        quote!(fn #ident(&self, #input) -> #rt::BoxFuture<'_, #output>)
    }

    /// The argument the method passes on: its parameter.
    fn argument(&self) -> TokenStream {
        let parameter = &self.parameter;
        quote!(#parameter)
    }

    pub(crate) fn sync_decl(&self) -> TokenStream {
        let (doc, signature) = (&self.doc, self.sync_signature());
        quote!(#doc #signature;)
    }

    pub(crate) fn async_decl(&self) -> TokenStream {
        let (doc, signature) = (&self.doc, self.async_signature(Capture::Trait));
        quote!(#doc #signature;)
    }

    pub(crate) fn erased_decl(&self) -> TokenStream {
        let signature = self.erased_signature();
        quote!(#signature;)
    }

    /// The erased trait's method for every `T` of the async trait: box the
    /// future, and the reply stream once the call resolves.
    pub(crate) fn erased_impl(&self) -> TokenStream {
        let (ident, rt, async_trait) = (&self.ident, self.rt, self.async_trait);
        let (signature, argument, reply) = (
            self.erased_signature(),
            self.argument(),
            self.reply_result(),
        );
        let body = if self.kind.streams_out() {
            quote! {
                ::std::boxed::Box::pin(async move {
                    let stream = <T as super::#async_trait>::#ident(self, #argument).await?;
                    ::core::result::Result::Ok(
                        ::std::boxed::Box::pin(stream) as #rt::BoxStream<'static, #reply>
                    )
                })
            }
        } else {
            quote!(::std::boxed::Box::pin(<T as super::#async_trait>::#ident(self, #argument)))
        };
        quote!(#signature { #body })
    }

    /// The handle's method: forward to the erased trait.
    pub(crate) fn dyn_impl(&self) -> TokenStream {
        let (ident, parameter) = (&self.ident, &self.parameter);
        let signature = self.async_signature(Capture::Nothing);
        let argument = if self.kind.streams_in() {
            quote!(::std::boxed::Box::pin(#parameter))
        } else {
            quote!(#parameter)
        };
        quote!(#signature { self.inner.#ident(#argument) })
    }

    /// `Inline<T>`: run the sync method inside the future's poll.
    pub(crate) fn inline_impl(&self) -> TokenStream {
        let (ident, sync_trait, parameter) = (&self.ident, self.sync_trait, &self.parameter);
        let signature = self.async_signature_as(Capture::BridgeParam, true);
        let buffer = if self.kind.streams_in() {
            quote!(let #parameter = Self::buffer(#parameter).await;)
        } else {
            TokenStream::new()
        };
        let argument = self.argument();
        let call = quote!(<T as #sync_trait>::#ident(Self::get_ref(self), #argument));
        let call = if self.kind.streams_out() {
            quote!(#call.map(Self::reply_stream))
        } else {
            call
        };
        quote!(#signature { #buffer #call })
    }

    /// `Offload<T>`: run the sync method on the blocking pool.
    pub(crate) fn offload_impl(&self) -> TokenStream {
        let (ident, sync_trait, parameter) = (&self.ident, self.sync_trait, &self.parameter);
        let signature = self.async_signature(Capture::BridgeParam);
        let body = match self.kind {
            Kind::Unary => quote! {
                Self::call(self, move |service| <T as #sync_trait>::#ident(service, #parameter))
            },
            Kind::ServerStreaming => quote! {
                Self::server_streaming(self, move |service| <T as #sync_trait>::#ident(service, #parameter))
            },
            Kind::ClientStreaming => quote! {
                Self::client_streaming(self, #parameter, <T as #sync_trait>::#ident)
            },
            Kind::Bidirectional => quote! {
                Self::bidirectional(self, #parameter, <T as #sync_trait>::#ident)
            },
        };
        quote!(#signature { #body })
    }

    /// A shared blocking implementation (`Arc<dyn <S>Sync>` in field `0`):
    /// call it.
    pub(crate) fn shared_impl(&self) -> TokenStream {
        let ident = &self.ident;
        let (signature, argument) = (self.sync_signature(), self.argument());
        quote!(#signature { self.0.#ident(#argument) })
    }

    /// `Blocking<T>`: block on the async method.
    pub(crate) fn blocking_impl(&self) -> TokenStream {
        let (ident, async_trait, parameter) = (&self.ident, self.async_trait, &self.parameter);
        let signature = self.sync_signature();
        let feed = if self.kind.streams_in() {
            quote!(let #parameter = Self::feed(self, #parameter);)
        } else {
            TokenStream::new()
        };
        let argument = self.argument();
        let call = quote!(<T as #async_trait>::#ident(Self::get_ref(self), #argument));
        let wait = if self.kind.streams_out() {
            quote!(Self::block_on_stream(self, #call))
        } else {
            quote!(Self::block_on(self, #call))
        };
        quote!(#signature { #feed #wait })
    }
}
